use {
    super::LookupTableCacheProvider,
    crate::{Result, SolanaRpcProvider},
    moka::future::Cache,
    solana_hash::Hash,
    solana_message::AddressLookupTableAccount,
    solana_pubkey::Pubkey,
    solana_rpc_client_api::response::RpcPrioritizationFee,
    solana_signature::Signature,
    tracing::{Level, enabled, event, info_span},
};

impl<T: SolanaRpcProvider> LookupTableCacheProvider<T> {
    pub fn new(
        client: T,
        lookup_cache: Cache<Pubkey, AddressLookupTableAccount>,
        negative_cache: Cache<Pubkey, ()>,
    ) -> Self {
        Self {
            inner: client,
            lookup_cache,
            negative_cache,
        }
    }

    /// Checks if the lookup table cache is empty.
    ///
    /// **Note:** This method does not run pending tasks on the caches.
    /// Results may be inaccurate.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lookup_cache.entry_count() == 0
    }

    /// Checks if the negative lookup table cache is empty.
    ///
    /// **Note:** This method does not run pending tasks on the caches.
    /// Results may be inaccurate.
    #[must_use]
    pub fn is_empty_negative(&self) -> bool {
        self.negative_cache.entry_count() == 0
    }

    #[must_use]
    /// Returns the total number of entries in the lookup table cache.
    ///
    /// **Note:** This method does not run pending tasks on the caches. Results
    /// may be inaccurate.
    pub fn len(&self) -> u64 {
        self.lookup_cache.entry_count()
    }

    #[must_use]
    /// **Note:** This method does not run pending tasks on the caches. Results
    /// may be inaccurate.
    pub fn len_negative(&self) -> u64 {
        self.negative_cache.entry_count()
    }

    /// Returns the total number of entries in both lookup table and negative
    /// caches.
    ///
    /// **Note:** This method does not run pending tasks on the caches. Results
    /// may be inaccurate.
    #[must_use]
    pub async fn total(&self) -> u64 {
        self.len() + self.len_negative()
    }

    /// Clears all lookup table and negative caches.
    pub async fn clear_all(&self) {
        self.clear_lookups().await;
        self.clear_negative().await;
    }

    pub async fn clear_lookups(&self) {
        self.lookup_cache.invalidate_all();
        self.lookup_cache.run_pending_tasks().await;
    }

    pub async fn clear_negative(&self) {
        self.negative_cache.invalidate_all();
        self.negative_cache.run_pending_tasks().await;
    }

    /// Runs pending tasks on both caches to ensure counts are accurate.
    /// This is needed because moka cache uses eventual consistency for
    /// entry_count.
    pub async fn sync(&self) {
        self.lookup_cache.run_pending_tasks().await;
        self.negative_cache.run_pending_tasks().await;
    }
}

impl<T: SolanaRpcProvider> LookupTableCacheProvider<T> {
    /// Helper function to fetch a single lookup table account with proper error
    /// handling
    async fn try_get_lookup_account(&self, pubkey: Pubkey) -> Result<AddressLookupTableAccount> {
        let span = if enabled!(Level::TRACE) {
            self.sync().await; // to get accurate cache stats
            let cached_lookups = self.len();
            let cached_negatives = self.len_negative();
            info_span!("lookup-resolver", lookup = ?pubkey, cached_lookups, cached_negatives)
        } else {
            info_span!("lookup-resolver", lookup = ?pubkey)
        };
        let _guard = span.enter();

        self.lookup_cache
            .try_get_with(pubkey, async {
                event!(Level::INFO, "cache-miss");
                let results = self.inner.get_lookup_table_accounts(&[pubkey]).await?;
                if results.is_empty() {
                    event!(Level::INFO, "no-lookup-table");
                    Err(crate::Error::LookupTableMiss)
                } else {
                    Ok(results[0].to_owned())
                }
            })
            .await
            .map_err(Self::handle_cache_error)
    }

    /// Converts moka cache Arc errors to application errors
    fn handle_cache_error(arc_err: std::sync::Arc<crate::Error>) -> crate::Error {
        match std::sync::Arc::try_unwrap(arc_err) {
            Ok(err) => {
                event!(Level::ERROR, "cache error: {err}");
                err
            }
            Err(arc) => {
                // Arc couldn't be unwrapped, extract the error type
                match &*arc {
                    crate::Error::LookupTableMiss => crate::Error::LookupTableMiss,
                    _ => crate::Error::MokaCacheError(arc.to_string()),
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl<T: SolanaRpcProvider + Send + Sync> SolanaRpcProvider for LookupTableCacheProvider<T> {
    async fn get_recent_prioritization_fees(
        &self,
        accounts: &[Pubkey],
    ) -> Result<Vec<RpcPrioritizationFee>> {
        self.inner.get_recent_prioritization_fees(accounts).await
    }

    /// Fetches lookup table accounts from the RPC client and caches them.
    ///
    /// **NOTE** the order of the results does not matter.
    /// If pubkeys = [A, B, C] and cache has [A, C]:
    /// result = [A, C]  // from cache
    /// result.extend(\[ B \])  // fetched
    /// Final: \[ A, C, B \]  // THIS IS VALID
    async fn get_lookup_table_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<AddressLookupTableAccount>> {
        let mut resolved = Vec::with_capacity(pubkeys.len());

        for &pubkey in pubkeys {
            match self.try_get_lookup_account(pubkey).await {
                Ok(account) => resolved.push(account),
                Err(crate::Error::LookupTableMiss) => self.negative_cache.insert(pubkey, ()).await,
                Err(err) => return Err(err),
            }
        }

        Ok(resolved)
    }

    async fn get_latest_blockhash(&self) -> Result<Hash> {
        self.inner.get_latest_blockhash().await
    }

    async fn simulate_transaction(
        &self,
        tx: &solana_transaction::versioned::VersionedTransaction,
        config: solana_rpc_client_api::config::RpcSimulateTransactionConfig,
    ) -> Result<solana_rpc_client_api::response::RpcSimulateTransactionResult> {
        self.inner.simulate_transaction(tx, config).await
    }

    async fn send_and_confirm_transaction(
        &self,
        tx: &solana_transaction::versioned::VersionedTransaction,
        config: Option<solana_rpc_client_api::config::RpcSendTransactionConfig>,
    ) -> Result<Signature> {
        self.inner.send_and_confirm_transaction(tx, config).await
    }
}

#[cfg(test)]
mod tests {

    use {
        super::*,
        crate::{
            SolanaRpcProvider,
            rpc::noop::{NoopRpc, NoopRpcNative},
        },
        dashmap::DashMap,
        solana_keypair::Keypair,
        solana_signer::Signer,
        std::{fmt::Debug, sync::Arc, time::Duration},
        tokio::time::sleep,
    };

    #[derive(Clone)]
    struct MockRpcProvider {
        inner: NoopRpcNative,
        lookups: Arc<DashMap<Pubkey, AddressLookupTableAccount>>,
    }

    impl Debug for MockRpcProvider {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "mocks={}", self.lookups.len())
        }
    }
    #[async_trait::async_trait]
    impl SolanaRpcProvider for MockRpcProvider {
        async fn get_recent_prioritization_fees(
            &self,
            accounts: &[Pubkey],
        ) -> Result<Vec<RpcPrioritizationFee>> {
            self.inner.get_recent_prioritization_fees(accounts).await
        }

        #[tracing::instrument(level = "info", skip(pubkeys) name = "mock_lookups")]
        async fn get_lookup_table_accounts(
            &self,
            pubkeys: &[Pubkey],
        ) -> Result<Vec<AddressLookupTableAccount>> {
            let mut result = Vec::new();
            for pubkey in pubkeys {
                if let Some(lookup) = self.lookups.get(pubkey) {
                    result.push(lookup.clone());
                }
            }
            Ok(result)
        }

        async fn get_latest_blockhash(&self) -> Result<Hash> {
            self.inner.get_latest_blockhash().await
        }

        async fn simulate_transaction(
            &self,
            tx: &solana_transaction::versioned::VersionedTransaction,
            config: solana_rpc_client_api::config::RpcSimulateTransactionConfig,
        ) -> Result<solana_rpc_client_api::response::RpcSimulateTransactionResult> {
            self.inner.simulate_transaction(tx, config).await
        }

        async fn send_and_confirm_transaction(
            &self,
            tx: &solana_transaction::versioned::VersionedTransaction,
            config: Option<solana_rpc_client_api::config::RpcSendTransactionConfig>,
        ) -> Result<Signature> {
            self.inner.send_and_confirm_transaction(tx, config).await
        }
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_lookup_cache() -> anyhow::Result<()> {
        let mock = MockRpcProvider {
            inner: NoopRpc::default(),
            lookups: Arc::new(DashMap::new()),
        };

        let lookup_cache = LookupTableCacheProvider::new(
            mock.clone(),
            Cache::builder()
                .time_to_live(Duration::from_millis(500))
                .build(),
            Cache::builder()
                .time_to_live(Duration::from_millis(500))
                .build(),
        );

        assert!(lookup_cache.is_empty());
        assert!(lookup_cache.is_empty_negative());

        assert_eq!(0, lookup_cache.len());
        assert_eq!(0, lookup_cache.len_negative());

        let hit1 = Keypair::new();
        let miss = Keypair::new();
        let hit2 = Keypair::new();

        let query = vec![hit1.pubkey(), miss.pubkey(), hit2.pubkey()];

        mock.lookups
            .insert(hit1.pubkey(), AddressLookupTableAccount {
                addresses: vec![hit1.pubkey()],
                key: hit1.pubkey(),
            });
        mock.lookups
            .insert(hit2.pubkey(), AddressLookupTableAccount {
                addresses: vec![hit2.pubkey()],
                key: hit2.pubkey(),
            });

        let results = lookup_cache.get_lookup_table_accounts(&query).await?;
        // Sync caches to ensure counts are accurate
        lookup_cache.sync().await;
        assert_eq!(2, results.len());
        assert_eq!(2, lookup_cache.len());
        assert_eq!(1, lookup_cache.len_negative());
        assert_eq!(3, lookup_cache.total().await);

        sleep(Duration::from_secs(1)).await;
        lookup_cache.sync().await;
        assert!(lookup_cache.is_empty());
        assert!(lookup_cache.is_empty_negative());
        let _ = lookup_cache.get_lookup_table_accounts(&query).await?;
        lookup_cache.sync().await;
        lookup_cache.clear_lookups().await;
        assert!(lookup_cache.is_empty());
        assert!(!lookup_cache.is_empty_negative());
        lookup_cache.clear_negative().await;
        assert!(lookup_cache.is_empty_negative());
        Ok(())
    }
}

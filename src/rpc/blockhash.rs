use {
    super::BlockHashCacheProvider,
    crate::{Result, SolanaRpcProvider},
    moka::future::Cache,
    solana_hash::Hash,
    solana_message::AddressLookupTableAccount,
    solana_pubkey::Pubkey,
    solana_rpc_client_api::response::RpcPrioritizationFee,
    solana_signature::Signature,
    std::time::Duration,
    tracing::{Level, event},
};

impl<T: SolanaRpcProvider> BlockHashCacheProvider<T> {
    pub fn new(client: T, ttl: Duration) -> Self {
        Self {
            inner: client,
            blockhash: Cache::builder().max_capacity(1).time_to_live(ttl).build(),
        }
    }
}

#[async_trait::async_trait]
impl<T: SolanaRpcProvider + Send + Sync> SolanaRpcProvider for BlockHashCacheProvider<T> {
    async fn get_latest_blockhash(&self) -> Result<Hash> {
        self.blockhash
            .try_get_with((), async {
                event!(Level::DEBUG, "blockhash cache miss");
                self.inner.get_latest_blockhash().await
            })
            .await
            .map_err(|arc_err| match std::sync::Arc::try_unwrap(arc_err) {
                Ok(e) => e,
                Err(arc) => crate::Error::MokaCacheError(arc.to_string()),
            })
    }

    async fn get_recent_prioritization_fees(
        &self,
        accounts: &[Pubkey],
    ) -> Result<Vec<RpcPrioritizationFee>> {
        self.inner.get_recent_prioritization_fees(accounts).await
    }

    async fn get_lookup_table_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<AddressLookupTableAccount>> {
        self.inner.get_lookup_table_accounts(pubkeys).await
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
    ) -> Result<Signature> {
        self.inner.send_and_confirm_transaction(tx).await
    }
}

#[cfg(test)]
mod tests {

    use {
        super::*,
        crate::{
            SolanaRpcProvider,
            TransactionBuilder,
            rpc::{CounterRpcProvider, NoopRpc},
        },
        solana_keypair::Keypair,
        solana_rpc_client_api::config::RpcSimulateTransactionConfig,
        solana_signer::Signer,
        tokio::time::sleep,
    };

    #[tokio::test]
    async fn test_blockhash_cache_provider() -> anyhow::Result<()> {
        let counter = CounterRpcProvider::new(NoopRpc);
        let hash_cache = BlockHashCacheProvider::new(counter.clone(), Duration::from_secs(1));
        hash_cache.get_latest_blockhash().await?;
        {
            assert_eq!(counter.get_counter(&crate::RpcMethod::Blockhash), 1);
        }
        sleep(Duration::from_millis(500)).await;
        hash_cache.get_latest_blockhash().await?;
        {
            assert_eq!(counter.get_counter(&crate::RpcMethod::Blockhash), 1);
        }
        sleep(Duration::from_millis(2000)).await;
        hash_cache.get_latest_blockhash().await?;
        assert_eq!(counter.get_counter(&crate::RpcMethod::Blockhash), 2);

        let _ = hash_cache
            .get_lookup_table_accounts(&[Pubkey::default()])
            .await?;
        let _ = hash_cache
            .get_recent_prioritization_fees(&[Pubkey::default()])
            .await?;

        let kp = Keypair::new();
        let tx: TransactionBuilder =
            spl_memo::build_memo(String::from("memo").as_bytes(), &[&kp.pubkey()]).into();
        let tx = tx.unsigned_tx(&kp.pubkey(), &hash_cache).await?;

        let _ = hash_cache
            .simulate_transaction(&tx, RpcSimulateTransactionConfig::default())
            .await?;

        let _ = hash_cache.send_and_confirm_transaction(&tx).await?;

        assert_eq!(1, counter.get_counter(&crate::RpcMethod::Fees));
        assert_eq!(1, counter.get_counter(&crate::RpcMethod::Lookup));
        assert_eq!(1, counter.get_counter(&crate::RpcMethod::Simulate));
        assert_eq!(1, counter.get_counter(&crate::RpcMethod::Send));

        counter.reset_counters();
        assert_eq!(0, counter.get_counter(&crate::RpcMethod::Fees));
        assert_eq!(0, counter.get_counter(&crate::RpcMethod::Lookup));
        assert_eq!(0, counter.get_counter(&crate::RpcMethod::Simulate));
        assert_eq!(0, counter.get_counter(&crate::RpcMethod::Send));
        Ok(())
    }
}

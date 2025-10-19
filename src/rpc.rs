mod native;
use {
    crate::{NativeRpcWrapper, Result, SolanaRpcProvider},
    solana_hash::Hash,
    solana_message::AddressLookupTableAccount,
    solana_pubkey::Pubkey,
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_rpc_client_api::response::RpcPrioritizationFee,
    solana_signature::Signature,
    std::{collections::HashMap, sync::Arc},
    tokio::sync::RwLock,
};

#[allow(dead_code)]
pub struct LookupTableCacheProvider {
    inner: NativeRpcWrapper,
    lookup_cache: Arc<RwLock<HashMap<Pubkey, AddressLookupTableAccount>>>,
}

impl From<RpcClient> for LookupTableCacheProvider {
    fn from(client: RpcClient) -> Self {
        Self::new(client)
    }
}

impl LookupTableCacheProvider {
    #[allow(dead_code)]
    pub fn new(client: RpcClient) -> Self {
        Self {
            inner: NativeRpcWrapper::from(client),
            lookup_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl SolanaRpcProvider for LookupTableCacheProvider {
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
        let mut result = Vec::with_capacity(pubkeys.len());
        let mut missing_keys = Vec::new();

        // Check cache for each pubkey
        {
            let cache = self.lookup_cache.read().await;
            for &pubkey in pubkeys {
                if let Some(cached) = cache.get(&pubkey) {
                    result.push(cached.clone());
                } else {
                    missing_keys.push(pubkey);
                }
            }
        }

        // Fetch missing keys
        if !missing_keys.is_empty() {
            let fetched = self.inner.get_lookup_table_accounts(&missing_keys).await?;

            // Cache the fetched results
            {
                let mut cache = self.lookup_cache.write().await;
                for (i, account) in fetched.iter().enumerate() {
                    cache.insert(missing_keys[i], account.clone());
                }
            }

            result.extend(fetched);
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
    ) -> Result<Signature> {
        self.inner.send_and_confirm_transaction(tx).await
    }
}

impl From<RpcClient> for NativeRpcWrapper {
    fn from(client: RpcClient) -> Self {
        Self(Arc::new(client))
    }
}

impl AsRef<RpcClient> for NativeRpcWrapper {
    fn as_ref(&self) -> &RpcClient {
        &self.0
    }
}

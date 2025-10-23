use {
    crate::{Result, SimpleCacheProvider, SolanaRpcProvider},
    solana_hash::Hash,
    solana_message::AddressLookupTableAccount,
    solana_pubkey::Pubkey,
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_rpc_client_api::response::RpcPrioritizationFee,
    solana_signature::Signature,
};

#[async_trait::async_trait]
impl<
    T: SolanaRpcProvider + AsRef<RpcClient> + Send + Sync,
    L: SolanaRpcProvider + Send + Sync,
    B: SolanaRpcProvider + Send + Sync,
> SolanaRpcProvider for SimpleCacheProvider<T, L, B>
{
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
        self.lookup_cache.get_lookup_table_accounts(pubkeys).await
    }

    async fn get_latest_blockhash(&self) -> Result<Hash> {
        self.blockhash_cache.get_latest_blockhash().await
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

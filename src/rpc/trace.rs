use {
    crate::{Result, SolanaRpcProvider, TraceRpcNativeProvider},
    base64::prelude::*,
    solana_hash::Hash,
    solana_message::AddressLookupTableAccount,
    solana_pubkey::Pubkey,
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_rpc_client_api::response::RpcPrioritizationFee,
    solana_signature::Signature,
    tracing::debug,
};

#[async_trait::async_trait]
impl<T: SolanaRpcProvider + AsRef<RpcClient> + Send + Sync + Clone> SolanaRpcProvider
    for TraceRpcNativeProvider<T>
{
    #[tracing::instrument(skip_all, level = tracing::Level::INFO)]
    async fn get_recent_prioritization_fees(
        &self,
        accounts: &[Pubkey],
    ) -> Result<Vec<RpcPrioritizationFee>> {
        self.0.get_recent_prioritization_fees(accounts).await
    }

    #[tracing::instrument(skip_all, level = tracing::Level::INFO)]
    async fn get_lookup_table_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<AddressLookupTableAccount>> {
        crate::lookup::fetch_lookup_tables(pubkeys, &self.0).await
    }

    #[tracing::instrument(skip_all, level = tracing::Level::INFO)]
    async fn get_latest_blockhash(&self) -> Result<Hash> {
        self.0.get_latest_blockhash().await
    }

    #[tracing::instrument(skip_all, level = tracing::Level::INFO)]
    async fn simulate_transaction(
        &self,
        tx: &solana_transaction::versioned::VersionedTransaction,
        config: solana_rpc_client_api::config::RpcSimulateTransactionConfig,
    ) -> Result<solana_rpc_client_api::response::RpcSimulateTransactionResult> {
        if tracing::enabled!(tracing::Level::DEBUG) {
            // This is safe to log, as this is sent on a PUBLIC blockchain for all to see.
            let transaction_base64 = BASE64_STANDARD.encode(bincode::serialize(&tx)?);
            debug!(simulate_tx =? transaction_base64);
        }
        self.0.simulate_transaction(tx, config).await
    }

    #[tracing::instrument(skip_all, level = tracing::Level::INFO)]
    async fn send_and_confirm_transaction(
        &self,
        tx: &solana_transaction::versioned::VersionedTransaction,
        config: Option<solana_rpc_client_api::config::RpcSendTransactionConfig>,
    ) -> Result<Signature> {
        self.0.send_and_confirm_transaction(tx, config).await
    }
}

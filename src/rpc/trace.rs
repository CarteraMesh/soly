use {
    crate::{Error, Result, SolanaRpcProvider, TraceNativeProvider},
    base64::prelude::*,
    solana_hash::Hash,
    solana_message::AddressLookupTableAccount,
    solana_pubkey::Pubkey,
    solana_rpc_client_api::response::RpcPrioritizationFee,
    solana_signature::Signature,
    tracing::debug,
};

#[async_trait::async_trait]
impl SolanaRpcProvider for TraceNativeProvider {
    #[tracing::instrument(skip_all, level = tracing::Level::INFO)]
    async fn get_recent_prioritization_fees(
        &self,
        accounts: &[Pubkey],
    ) -> Result<Vec<RpcPrioritizationFee>> {
        self.0
            .get_recent_prioritization_fees(accounts)
            .await
            .map_err(|e| Error::SolanaRpcError(format!("failed to get prioritization fees: {e}")))
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
        self.0
            .get_latest_blockhash()
            .await
            .map_err(|e| Error::SolanaRpcError(format!("failed to get latest blockhash: {e}")))
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
        let result = self
            .0
            .simulate_transaction_with_config(tx, config)
            .await
            .map_err(|e| Error::SolanaRpcError(format!("failed to simulate transaction: {e}")))?;
        if let Some(e) = result.value.err {
            let logs = result.value.logs.unwrap_or(Vec::new());
            let transaction_base64 = BASE64_STANDARD.encode(bincode::serialize(&tx)?);
            let msg = format!("{e}\nbase64: {transaction_base64}\n{}", logs.join("\n"));
            return Err(Error::SolanaSimulateFailure(msg));
        }
        Ok(result.value)
    }

    #[tracing::instrument(skip_all, level = tracing::Level::INFO)]
    async fn send_and_confirm_transaction(
        &self,
        tx: &solana_transaction::versioned::VersionedTransaction,
    ) -> Result<Signature> {
        self.0
            .send_and_confirm_transaction(tx)
            .await
            .map_err(|e| Error::SolanaRpcError(format!("failed to send transaction: {e}")))
    }
}

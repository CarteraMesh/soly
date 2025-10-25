use {
    crate::{Error, Result, TransactionRpcProvider},
    base64::prelude::*,
    solana_hash::Hash,
    solana_message::AddressLookupTableAccount,
    solana_pubkey::Pubkey,
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_rpc_client_api::response::RpcPrioritizationFee,
    solana_signature::Signature,
    tracing::{debug, trace},
};

#[async_trait::async_trait]
impl TransactionRpcProvider for std::sync::Arc<RpcClient> {
    async fn get_recent_prioritization_fees(
        &self,
        accounts: &[Pubkey],
    ) -> Result<Vec<RpcPrioritizationFee>> {
        debug!(accounts =? accounts.len(), "get_recent_prioritization_fees");
        self.as_ref()
            .get_recent_prioritization_fees(accounts)
            .await
            .map_err(|e| Error::SolanaRpcError(format!("failed to get prioritization fees: {e}")))
    }

    async fn get_lookup_table_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<AddressLookupTableAccount>> {
        debug!(accounts =? pubkeys.len(), "calling get_lookup_table_accounts");
        crate::lookup::fetch_lookup_tables(pubkeys, &self).await
    }

    async fn get_latest_blockhash(&self) -> Result<Hash> {
        debug!("calling get_latest_blockhash");
        self.as_ref()
            .get_latest_blockhash()
            .await
            .map_err(|e| Error::SolanaRpcError(format!("failed to get latest blockhash: {e}")))
    }

    async fn simulate_transaction(
        &self,
        tx: &solana_transaction::versioned::VersionedTransaction,
        config: solana_rpc_client_api::config::RpcSimulateTransactionConfig,
    ) -> Result<solana_rpc_client_api::response::RpcSimulateTransactionResult> {
        let result = self
            .as_ref()
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

    async fn send_and_confirm_transaction(
        &self,
        tx: &solana_transaction::versioned::VersionedTransaction,
        config: Option<solana_rpc_client_api::config::RpcSendTransactionConfig>,
    ) -> Result<Signature> {
        if tracing::enabled!(tracing::Level::TRACE) {
            let transaction_base64 = BASE64_STANDARD.encode(bincode::serialize(&tx)?);
            trace!(send_tx =? transaction_base64);
        }
        match config {
            None => self
                .as_ref()
                .send_and_confirm_transaction(tx)
                .await
                .map_err(|e| Error::SolanaRpcError(format!("failed to send transaction: {e}"))),
            Some(config) => {
                let result = self
                    .as_ref()
                    .send_transaction_with_config(tx, config)
                    .await
                    .map_err(|e| {
                        Error::SolanaRpcError(format!("failed to send transaction: {e}"))
                    })?;

                match self.as_ref().confirm_transaction(&result).await {
                    Err(e) => Err(Error::SolanaRpcError(format!(
                        "failed to confirm transaction: {result} Error:{e}"
                    ))),
                    Ok(t) => {
                        if t {
                            Ok(result)
                        } else {
                            Err(Error::SolanaRpcError(format!(
                                "Transaction is not confirmed: {result}"
                            )))
                        }
                    }
                }
            }
        }
    }
}

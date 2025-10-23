use {
    super::RpcMethod,
    crate::{CounterRpcProvider, Result, SolanaRpcProvider},
    solana_hash::Hash,
    solana_message::AddressLookupTableAccount,
    solana_pubkey::Pubkey,
    solana_rpc_client_api::response::RpcPrioritizationFee,
    solana_signature::Signature,
    std::fmt::Display,
};

impl<T: SolanaRpcProvider + Clone> Display for CounterRpcProvider<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Method Counters: blockhash={} fees={} lookup={} simulate={} send={}",
            self.get_counter(&RpcMethod::Blockhash),
            self.get_counter(&RpcMethod::Fees),
            self.get_counter(&RpcMethod::Lookup),
            self.get_counter(&RpcMethod::Simulate),
            self.get_counter(&RpcMethod::Send)
        )
    }
}

impl<T: SolanaRpcProvider + Clone> CounterRpcProvider<T> {
    /// Get the counter for a given method
    pub fn get_counter(&self, method: &RpcMethod) -> u64 {
        match self.counters.get(method) {
            Some(counter) => *counter,
            None => 0, /* this should never execute, as all methods are accounted for, and the
                        * CounterRpcProvider is initialized with all methods */
        }
    }

    pub fn reset_counters(&self) {
        for mut counter in self.counters.iter_mut() {
            *counter.value_mut() = 0;
        }
    }
}

#[async_trait::async_trait]
impl<T: SolanaRpcProvider + Send + Sync + Clone> SolanaRpcProvider for CounterRpcProvider<T> {
    async fn get_recent_prioritization_fees(
        &self,
        accounts: &[Pubkey],
    ) -> Result<Vec<RpcPrioritizationFee>> {
        *self.counters.get_mut(&RpcMethod::Fees).unwrap() += 1;
        self.inner.get_recent_prioritization_fees(accounts).await
    }

    async fn get_lookup_table_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<AddressLookupTableAccount>> {
        *self.counters.get_mut(&RpcMethod::Lookup).unwrap() += 1;
        self.inner.get_lookup_table_accounts(pubkeys).await
    }

    async fn get_latest_blockhash(&self) -> Result<Hash> {
        *self.counters.get_mut(&RpcMethod::Blockhash).unwrap() += 1;
        self.inner.get_latest_blockhash().await
    }

    async fn simulate_transaction(
        &self,
        tx: &solana_transaction::versioned::VersionedTransaction,
        config: solana_rpc_client_api::config::RpcSimulateTransactionConfig,
    ) -> Result<solana_rpc_client_api::response::RpcSimulateTransactionResult> {
        *self.counters.get_mut(&RpcMethod::Simulate).unwrap() += 1;
        self.inner.simulate_transaction(tx, config).await
    }

    async fn send_and_confirm_transaction(
        &self,
        tx: &solana_transaction::versioned::VersionedTransaction,
    ) -> Result<Signature> {
        *self.counters.get_mut(&RpcMethod::Send).unwrap() += 1;
        self.inner.send_and_confirm_transaction(tx).await
    }
}

mod blockhash;
mod counter;
mod lookup;
mod native;
mod trace;
use {
    crate::{Result, SolanaRpcProvider},
    dashmap::DashMap,
    moka::future::Cache,
    solana_hash::Hash,
    solana_message::AddressLookupTableAccount,
    solana_pubkey::Pubkey,
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_rpc_client_api::response::{RpcPrioritizationFee, RpcSimulateTransactionResult},
    solana_signature::Signature,
    std::sync::Arc,
};

/// Provider with lookup table caching.
#[derive(bon::Builder)]
pub struct LookupTableCacheProvider<T: SolanaRpcProvider> {
    inner: T,
    lookup_cache: Cache<Pubkey, AddressLookupTableAccount>,
    negative_cache: Cache<Pubkey, ()>,
}

pub struct BlockHashCacheProvider<T: SolanaRpcProvider> {
    inner: T,
    blockhash: Cache<(), Hash>,
}

/// A thread-safe wrapper around Solana's native RPC client
///
/// This wrapper uses `Arc` internally for efficient cloning and shared
/// ownership. Use this when you need a type that implements the
/// `SolanaRpcProvider` trait while working with the native Solana RPC client.
pub struct NativeRpcWrapper(pub Arc<RpcClient>);

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

/// A thread-safe tracing wrapper around Solana's native RPC client
pub struct TraceNativeProvider(pub Arc<RpcClient>);

impl From<RpcClient> for TraceNativeProvider {
    fn from(client: RpcClient) -> Self {
        Self(Arc::new(client))
    }
}

impl AsRef<RpcClient> for TraceNativeProvider {
    fn as_ref(&self) -> &RpcClient {
        &self.0
    }
}

/// Convenient definitions for the [`CounterRpcProvider`]
#[derive(Eq, std::hash::Hash, PartialEq, PartialOrd)]
pub enum RpcMethod {
    Blockhash,
    Lookup,
    Simulate,
    Send,
    Fees,
}

/// A testing utility which implements a simple counter for tracking RPC method
/// calls.
///
/// **NOTE**: not meant for production use
///
/// This provider is useful for testing and debugging purposes
pub struct CounterRpcProvider<T: SolanaRpcProvider> {
    inner: T,
    pub(super) counters: DashMap<RpcMethod, u64>,
}

impl<T: SolanaRpcProvider> From<T> for CounterRpcProvider<T> {
    fn from(inner: T) -> Self {
        Self::new(inner)
    }
}

impl<T: SolanaRpcProvider> CounterRpcProvider<T> {
    pub fn new(inner: T) -> Self {
        let counters = DashMap::new();
        counters.insert(RpcMethod::Blockhash, 0);
        counters.insert(RpcMethod::Lookup, 0);
        counters.insert(RpcMethod::Simulate, 0);
        counters.insert(RpcMethod::Send, 0);
        counters.insert(RpcMethod::Fees, 0);
        Self { inner, counters }
    }
}

#[async_trait::async_trait]
impl<T: SolanaRpcProvider + Send + Sync> SolanaRpcProvider for Arc<T> {
    async fn get_recent_prioritization_fees(
        &self,
        accounts: &[Pubkey],
    ) -> Result<Vec<RpcPrioritizationFee>> {
        (**self).get_recent_prioritization_fees(accounts).await
    }

    async fn get_lookup_table_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<AddressLookupTableAccount>> {
        (**self).get_lookup_table_accounts(pubkeys).await
    }

    async fn get_latest_blockhash(&self) -> Result<Hash> {
        (**self).get_latest_blockhash().await
    }

    async fn simulate_transaction(
        &self,
        tx: &solana_transaction::versioned::VersionedTransaction,
        config: solana_rpc_client_api::config::RpcSimulateTransactionConfig,
    ) -> Result<RpcSimulateTransactionResult> {
        (**self).simulate_transaction(tx, config).await
    }

    async fn send_and_confirm_transaction(
        &self,
        tx: &solana_transaction::versioned::VersionedTransaction,
    ) -> Result<Signature> {
        (**self).send_and_confirm_transaction(tx).await
    }
}

pub struct NoopRpc;

#[allow(unused_variables)]
mod noop {
    use super::*;
    #[async_trait::async_trait]
    impl SolanaRpcProvider for NoopRpc {
        async fn get_recent_prioritization_fees(
            &self,
            accounts: &[Pubkey],
        ) -> Result<Vec<RpcPrioritizationFee>> {
            Ok(vec![])
        }

        async fn get_lookup_table_accounts(
            &self,
            pubkeys: &[Pubkey],
        ) -> Result<Vec<AddressLookupTableAccount>> {
            Ok(vec![])
        }

        async fn get_latest_blockhash(&self) -> Result<Hash> {
            Ok(Hash::new_unique())
        }

        async fn simulate_transaction(
            &self,
            tx: &solana_transaction::versioned::VersionedTransaction,
            config: solana_rpc_client_api::config::RpcSimulateTransactionConfig,
        ) -> Result<solana_rpc_client_api::response::RpcSimulateTransactionResult> {
            Ok(RpcSimulateTransactionResult {
                err: None,
                logs: None,
                accounts: None,
                units_consumed: None,
                loaded_accounts_data_size: None,
                return_data: None,
                inner_instructions: None,
                replacement_blockhash: None,
            })
        }

        async fn send_and_confirm_transaction(
            &self,
            tx: &solana_transaction::versioned::VersionedTransaction,
        ) -> Result<Signature> {
            Ok(Signature::default())
        }
    }
}

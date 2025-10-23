mod blockhash;
mod counter;
mod lookup;
mod native;
mod simple;
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

/// Combined cache provider with lookup table and blockhash caching.
///
/// This provider combines [`LookupTableCacheProvider`] and
/// [`BlockHashCacheProvider`] to provide comprehensive caching for Solana RPC
/// operations. It uses `Arc` wrappers to enable efficient cloning while sharing
/// cache state across instances.
///
/// ## Generic Type Parameters
///
/// The three generic parameters allow flexible composition:
/// - `T`: Main provider for non-cached operations (fees, simulation, sending
///   transactions)
/// - `L`: Provider used by the lookup table cache (can be same as `T` via
///   clone)
/// - `B`: Provider used by the blockhash cache (can be same as `T` via clone)
///
/// This design allows you to:
/// - Use the same provider for all operations: `SimpleCacheProvider<Client,
///   Client, Client>`
/// - Use different providers with different configurations (timeouts, retries,
///   etc.)
/// - Mix cached and non-cached providers as needed
///
/// # NOTE
///
/// This implementation is intended for demonstration and testing purposes only.
/// It lacks production-ready features such as:
/// - Request retries on failures
/// - Rate limiting and throttling
/// - Circuit breaker patterns
/// - Comprehensive error handling
///
/// # Example
///
/// ```rust,no_run
/// use {
///     moka::future::Cache,
///     soly::rpc::{BlockHashCacheProvider, LookupTableCacheProvider, SimpleCacheProvider},
///     std::{sync::Arc, time::Duration},
/// };
///
/// // Using the same RPC client for all operations
/// let rpc_client = /* your RPC client */;
///
/// let lookup_cache = Arc::new(
///     LookupTableCacheProvider::builder()
///         .inner(rpc_client.clone())  // Same client, cloned for lookup cache
///         .lookup_cache(
///             Cache::builder()
///                 .time_to_live(Duration::from_secs(60))
///                 .build(),
///         )
///         .negative_cache(
///             Cache::builder()
///                 .time_to_live(Duration::from_secs(60))
///                 .build(),
///         )
///         .build(),
/// );
///
/// let blockhash_cache = Arc::new(BlockHashCacheProvider::new(
///     rpc_client.clone(),  // Same client, cloned for blockhash cache
///     Duration::from_secs(20),
/// ));
///
/// let cached_provider = SimpleCacheProvider::builder()
///     .inner(rpc_client)  // Same client for non-cached operations
///     .lookup_cache(lookup_cache)
///     .blockhash_cache(blockhash_cache)
///     .build();
/// ```
#[derive(Clone, bon::Builder)]
pub struct SimpleCacheProvider<T: SolanaRpcProvider, L: SolanaRpcProvider, B: SolanaRpcProvider> {
    inner: T,
    lookup_cache: Arc<LookupTableCacheProvider<L>>,
    blockhash_cache: Arc<BlockHashCacheProvider<B>>,
}

/// Provider with lookup table caching.
///
/// This uses [`moka::future::Cache`] for efficient caching of lookup tables.
/// See their documentation for more details.
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
#[derive(Clone)]
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
#[derive(Clone)]
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
#[derive(Clone)]
pub struct CounterRpcProvider<T: SolanaRpcProvider + Clone> {
    inner: T,
    pub(super) counters: Arc<DashMap<RpcMethod, u64>>,
}

impl<T: SolanaRpcProvider + Clone> From<T> for CounterRpcProvider<T> {
    fn from(inner: T) -> Self {
        Self::new(inner)
    }
}

impl<T: SolanaRpcProvider + Clone> CounterRpcProvider<T> {
    pub fn new(inner: T) -> Self {
        let counters = Arc::new(DashMap::new());
        counters.insert(RpcMethod::Blockhash, 0);
        counters.insert(RpcMethod::Lookup, 0);
        counters.insert(RpcMethod::Simulate, 0);
        counters.insert(RpcMethod::Send, 0);
        counters.insert(RpcMethod::Fees, 0);
        Self { inner, counters }
    }
}

#[derive(Clone)]
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

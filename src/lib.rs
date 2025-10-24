#![doc = include_str!("../README.md")]

mod error;
mod fee;
mod lookup;
mod rpc;
mod transaction;
use {
    borsh::BorshSerialize,
    solana_hash::Hash,
    solana_instruction::Instruction,
    solana_message::AddressLookupTableAccount,
    solana_pubkey::Pubkey,
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_rpc_client_api::response::RpcPrioritizationFee,
    solana_signature::Signature,
};
pub use {
    error::*,
    fee::CalcFeeResult,
    lookup::*,
    moka::{self, future::Cache},
    nitrogen_instruction_builder::*,
    rpc::*,
    transaction::*,
};
pub type Result<T> = std::result::Result<T, Error>;

pub trait InstructionBuilderExt {
    fn tx(self) -> TransactionBuilder;
}

impl<T: BorshSerialize> InstructionBuilderExt for InstructionBuilder<T> {
    fn tx(self) -> TransactionBuilder {
        self.into()
    }
}

/// Trait abstracting RPC operations for Solana transactions.
///
/// This trait allows for different RPC implementations (native, cached, mocked)
/// while maintaining a consistent interface for transaction building and
/// submission.
///
/// # Implementing Custom Providers
/// Custom implementations can add caching, rate limiting, retry logic, or
/// use alternative RPC endpoints. All implementations must handle errors by
/// converting them to the crate's [`Error`] type.
///
/// # Examples
/// ```no_run
/// # use soly::{SolanaRpcProvider, NativeRpcWrapper};
/// # use solana_rpc_client::nonblocking::rpc_client::RpcClient;
/// let rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
/// let provider: NativeRpcWrapper = rpc.into();
/// ```
#[async_trait::async_trait]
pub trait SolanaRpcProvider: Send + Sync {
    async fn get_recent_prioritization_fees(
        &self,
        accounts: &[Pubkey],
    ) -> Result<Vec<RpcPrioritizationFee>>;
    async fn get_lookup_table_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<AddressLookupTableAccount>>;
    async fn get_latest_blockhash(&self) -> Result<Hash>;
    async fn simulate_transaction(
        &self,
        tx: &solana_transaction::versioned::VersionedTransaction,
        config: solana_rpc_client_api::config::RpcSimulateTransactionConfig,
    ) -> Result<solana_rpc_client_api::response::RpcSimulateTransactionResult>;
    async fn send_and_confirm_transaction(
        &self,
        tx: &solana_transaction::versioned::VersionedTransaction,
        config: Option<solana_rpc_client_api::config::RpcSendTransactionConfig>,
    ) -> Result<Signature>;
}

/// Provides access to native [`RpcClient`] for composability
pub trait SolanaRpcProviderNative: SolanaRpcProvider + AsRef<RpcClient> {}

impl From<Instruction> for TransactionBuilder {
    fn from(instruction: Instruction) -> Self {
        Self::builder().instructions(vec![instruction]).build()
    }
}

impl<T: BorshSerialize> From<InstructionBuilder<T>> for TransactionBuilder {
    fn from(builder: InstructionBuilder<T>) -> Self {
        Self::from(builder.into_instruction())
    }
}

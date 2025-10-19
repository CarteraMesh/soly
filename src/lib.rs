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
    std::sync::Arc,
};
pub use {
    error::*,
    fee::CalcFeeResult,
    lookup::*,
    nitrogen_instruction_builder::*,
    transaction::*,
};
pub type Result<T> = std::result::Result<T, Error>;

pub struct NativeRpcWrapper(Arc<RpcClient>);

#[cfg(not(feature = "blocking"))]
#[async_trait::async_trait]
pub trait SolanaRpcProvider {
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
    ) -> Result<Signature>;
}

#[cfg(feature = "blocking")]
pub trait SolanaRpcProvider {
    fn get_recent_prioritization_fees(
        &self,
        accounts: &[Pubkey],
    ) -> Result<Vec<RpcPrioritizationFee>>;
    async fn get_lookup_table_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<AddressLookupTableAccount>>;
    fn get_latest_blockhash(&self) -> Result<Hash>;
    fn simulate_transaction(
        &self,
        tx: &solana_transaction::versioned::VersionedTransaction,
        config: solana_rpc_client_api::config::RpcSimulateTransactionConfig,
    ) -> Result<solana_rpc_client_api::response::RpcSimulateTransactionResult>;
    fn send_and_confirm_transaction(
        &self,
        tx: &solana_transaction::versioned::VersionedTransaction,
    ) -> Result<Signature>;
}

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

#![doc = include_str!("../README.md")]

mod error;
mod fee;
mod lookup;
mod transaction;
use {borsh::BorshSerialize, solana_instruction::Instruction};
pub use {error::*, lookup::*, nitrogen_instruction_builder::*, transaction::*};
pub type Result<T> = std::result::Result<T, Error>;

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

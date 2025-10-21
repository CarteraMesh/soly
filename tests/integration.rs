mod common;
use {
    borsh::BorshSerialize,
    common::*,
    solana_instruction::AccountMeta,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    soly::{InstructionBuilder, InstructionBuilderExt, TransactionBuilder},
    tracing::info,
};

#[derive(BorshSerialize)]
pub struct MemoData {
    pub memo: Vec<u8>,
}

impl From<&str> for MemoData {
    fn from(value: &str) -> Self {
        MemoData {
            memo: value.to_string().into_bytes(),
        }
    }
}

#[tokio::test]
async fn test_instruction_builder() -> anyhow::Result<()> {
    let (kp, rpc) = init()?;
    let memo: MemoData = "Hello, World!".into();
    let accounts = vec![AccountMeta::new_readonly(kp.pubkey(), true)];
    let sig = InstructionBuilder::builder()
        .program_id(spl_memo::id())
        .accounts(accounts)
        .params(memo)
        .build()
        .tx()
        .send(&rpc, &kp.pubkey(), &[&kp])
        .await?;
    info!("{sig}");
    Ok(())
}

#[tokio::test]
async fn test_transaction_builder_single_instruction() -> anyhow::Result<()> {
    let (kp, rpc) = init()?;
    let memo: MemoData = "Single instruction test".into();
    let accounts = vec![AccountMeta::new_readonly(kp.pubkey(), true)];

    let instruction_builder = InstructionBuilder::builder()
        .program_id(spl_memo::id())
        .accounts(accounts)
        .params(memo)
        .build();

    let tx = TransactionBuilder::builder()
        .instructions(vec![])
        .build()
        .push(instruction_builder);

    let sig = tx.send(&rpc, &kp.pubkey(), &[&kp]).await?;
    info!("Single instruction tx: {sig}");
    Ok(())
}

#[tokio::test]
async fn test_transaction_builder_multiple_instructions() -> anyhow::Result<()> {
    let (kp, rpc) = init()?;
    let accounts = vec![AccountMeta::new_readonly(kp.pubkey(), true)];

    let memo1: MemoData = "First memo".into();
    let memo2: MemoData = "Second memo".into();
    let memo3: MemoData = "Third memo".into();

    let builders = vec![
        InstructionBuilder::builder()
            .program_id(spl_memo::id())
            .accounts(accounts.clone())
            .params(memo1)
            .build(),
        InstructionBuilder::builder()
            .program_id(spl_memo::id())
            .accounts(accounts.clone())
            .params(memo2)
            .build(),
        InstructionBuilder::builder()
            .program_id(spl_memo::id())
            .accounts(accounts.clone())
            .params(memo3)
            .build(),
    ];

    let tx = TransactionBuilder::builder()
        .instructions(vec![])
        .build()
        .append(builders);

    let sig = tx.send(&rpc, &kp.pubkey(), &[&kp]).await?;
    info!("Multiple instructions tx: {sig}");
    Ok(())
}

#[test]
fn test_remaining_accounts() {
    let memo: MemoData = "With remaining accounts".into();
    let base_accounts = vec![AccountMeta::new_readonly(Pubkey::new_unique(), true)];
    let remaining_accounts = vec![
        AccountMeta::new_readonly(Pubkey::new_unique(), false),
        AccountMeta::new_readonly(Pubkey::new_unique(), false),
    ];

    let instruction_builder = InstructionBuilder::builder()
        .program_id(spl_memo::id())
        .accounts(base_accounts.clone())
        .params(memo)
        .build()
        .remaining_accounts(remaining_accounts.clone());

    let instruction = instruction_builder.instruction();

    // Verify the instruction has all accounts (base + remaining)
    assert_eq!(
        instruction.accounts.len(),
        base_accounts.len() + remaining_accounts.len()
    );
    assert_eq!(instruction.program_id, spl_memo::id());
}

#[tokio::test]
async fn test_empty_memo() -> anyhow::Result<()> {
    let (kp, rpc) = init()?;
    let memo: MemoData = "".into();
    let accounts = vec![AccountMeta::new_readonly(kp.pubkey(), true)];

    let instruction_builder = InstructionBuilder::builder()
        .program_id(spl_memo::id())
        .accounts(accounts)
        .params(memo)
        .build();

    let sig = TransactionBuilder::from(instruction_builder)
        .send(&rpc, &kp.pubkey(), &[&kp])
        .await?;
    info!("Empty memo tx: {sig}");
    Ok(())
}

#[test]
fn test_instruction_creation() {
    let memo: MemoData = "Test instruction creation".into();
    let accounts = vec![AccountMeta::new_readonly(Pubkey::new_unique(), true)];

    let builder = InstructionBuilder::builder()
        .program_id(spl_memo::id())
        .accounts(accounts.clone())
        .params(memo)
        .build();

    let instruction = builder.instruction();
    assert_eq!(instruction.program_id, spl_memo::id());
    assert_eq!(instruction.accounts.len(), accounts.len());
}

#[test]
fn test_transaction_builder_creation() {
    let memo: MemoData = "Test transaction creation".into();
    let accounts = vec![AccountMeta::new_readonly(Pubkey::new_unique(), true)];

    let builder = InstructionBuilder::builder()
        .program_id(spl_memo::id())
        .accounts(accounts)
        .params(memo)
        .build();

    let tx: TransactionBuilder = builder.into();
    assert_eq!(tx.instructions.len(), 1);
    assert_eq!(tx.instructions[0].program_id, spl_memo::id());
}

#[test]
fn test_extend_instruction() {
    let memo: MemoData = "Test extend".into();
    let accounts = vec![AccountMeta::new_readonly(Pubkey::new_unique(), true)];

    let ix = InstructionBuilder::builder()
        .program_id(spl_memo::id())
        .accounts(accounts)
        .params(memo)
        .build()
        .instruction();

    let mut tx = TransactionBuilder::default();
    tx.extend(vec![ix.clone(), ix.clone()]);

    assert_eq!(tx.instructions.len(), 2);
    assert_eq!(tx.instructions[0].program_id, spl_memo::id());
    assert_eq!(tx.instructions[1].program_id, spl_memo::id());
}

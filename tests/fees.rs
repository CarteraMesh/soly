mod common;
use {
    common::*,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    soly::TransactionBuilder,
    tracing::info,
};

fn builder(payer: &Pubkey) -> TransactionBuilder {
    let tx: TransactionBuilder = random_instructions(payer).into();
    tx.with_memo("github.com/carteraMesh", &[payer])
        .with_memo("soly", &[payer])
}

#[tokio::test]
async fn test_fee_with_default_percentile() -> anyhow::Result<()> {
    let (kp, rpc) = init()?;
    let span = tracing::info_span!("fee_with_default_percentile");
    let _g = span.enter();
    let payer = kp.pubkey();
    let tx = builder(&payer)
        .with_priority_fees(
            &payer,
            &rpc,
            &[
                solana_system_interface::program::ID,
                spl_memo_interface::v3::id(),
            ],
            1_000_000,
            None,
        )
        .await?;

    assert!(tx.instructions[0].program_id == solana_compute_budget_interface::ID);
    assert!(tx.instructions[1].program_id == solana_compute_budget_interface::ID);
    let sig = tx.send(&rpc, &payer, &[&kp]).await?;
    info!(
        sig =? sig
    );
    Ok(())
}

#[tokio::test]
async fn test_fee_with_max_priority() -> anyhow::Result<()> {
    let (kp, rpc) = init()?;
    let span = tracing::info_span!("fee_with_max_priority");
    let _g = span.enter();
    let payer = kp.pubkey();
    let tx = builder(&payer)
        .with_priority_fees(
            &payer,
            &rpc,
            &[
                solana_system_interface::program::ID,
                spl_memo_interface::v3::ID,
            ],
            u64::MAX,
            None,
        )
        .await?;
    assert_eq!(
        7,
        tx.instructions.len(),
        "size of instructions are not the same"
    );
    assert!(tx.instructions[0].program_id == solana_compute_budget_interface::ID);
    assert!(tx.instructions[1].program_id == solana_compute_budget_interface::ID);
    let sig = tx.send(&rpc, &payer, &[&kp]).await?;
    info!(
        sig = ?sig
    );
    Ok(())
}
#[tokio::test]
async fn test_fee_prepend() -> anyhow::Result<()> {
    let (kp, rpc) = init()?;
    let span = tracing::info_span!("fee_prepend");
    let _g = span.enter();
    let payer = kp.pubkey();
    let tx = builder(&payer).prepend_compute_budget_instructions(1_000_000, 200_000)?;

    assert_eq!(
        7,
        tx.instructions.len(),
        "size of instructions are not the same"
    );
    assert!(tx.instructions[0].program_id == solana_compute_budget_interface::ID);
    assert!(tx.instructions[1].program_id == solana_compute_budget_interface::ID);
    let sig = tx.send(&rpc, &payer, &[&kp]).await?;
    info!(
        sig = ?sig
    );
    let result = tx.clone().prepend_compute_budget_instructions(1, 2);
    assert!(result.is_err());
    let result = tx.prepend_compute_budget_instructions(u32::MAX, 2);
    assert!(result.is_err());
    Ok(())
}

#[tokio::test]
async fn test_fee_with_priority_fees() -> anyhow::Result<()> {
    let (kp, rpc) = init()?;
    let span = tracing::info_span!("fee_with_priority");
    let _g = span.enter();
    let payer = kp.pubkey();
    let tx = builder(&payer)
        .with_priority_fees(
            &payer,
            &rpc,
            &[
                solana_system_interface::program::ID,
                spl_memo_interface::v3::ID,
            ],
            1_000_000,
            Some(50),
        )
        .await?;

    assert_eq!(
        7,
        tx.instructions.len(),
        "size of instructions are not the same"
    );
    assert!(tx.instructions[0].program_id == solana_compute_budget_interface::ID);
    assert!(tx.instructions[1].program_id == solana_compute_budget_interface::ID);

    let sig = tx.send(&rpc, &payer, &[&kp]).await?;
    info!(sig = ?sig);

    Ok(())
}

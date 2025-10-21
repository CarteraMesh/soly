mod common;
use {
    common::*,
    moka::future::Cache,
    solana_keypair::Keypair,
    solana_signer::Signer,
    soly::{
        BlockHashCacheProvider,
        LookupTableCacheProvider,
        SolanaRpcProvider,
        TransactionBuilder,
    },
    std::time::Duration,
    tokio::time::sleep,
    tracing::{info, info_span},
};

#[tokio::test]
async fn test_latest_blockhash_cache() -> anyhow::Result<()> {
    let (kp, rpc) = init()?;
    let span = info_span!("test_latest_blockhash_cache");
    let _guard = span.enter();
    info!("starting test");
    let rpc = BlockHashCacheProvider::new(rpc, Duration::from_secs(1));
    let hash = rpc.get_latest_blockhash().await?;
    info!("sleeping");
    sleep(Duration::from_millis(200)).await;
    assert_eq!(hash, rpc.get_latest_blockhash().await?);
    info!("sleeping");
    sleep(Duration::from_millis(1000)).await;
    assert!(hash != rpc.get_latest_blockhash().await?);
    let tx = TransactionBuilder::default()
        .with_memo(MEMO_PKG, &[&kp.pubkey()])
        .with_priority_fees(
            &kp.pubkey(),
            &rpc,
            &[solana_system_interface::program::ID],
            1_000_000,
            None,
        )
        .await?
        .with_lookup_keys([TEST_LOOKUP_TABLE_ADDRESS]);
    let sig = tx.send(&rpc, &kp.pubkey(), &[&kp]).await?;
    info!("{sig}");
    Ok(())
}

#[tokio::test]
async fn test_lookup_cache() -> anyhow::Result<()> {
    let (kp, rpc) = init()?;
    let span = info_span!("test_lookup_cache");
    let _guard = span.enter();
    info!("starting test");
    let rpc = LookupTableCacheProvider::builder()
        .inner(rpc)
        .lookup_cache(
            Cache::builder()
                .time_to_live(Duration::from_secs(1))
                .build(),
        )
        .negative_cache(
            Cache::builder()
                .time_to_live(Duration::from_secs(1))
                .build(),
        )
        .build();

    let random = Keypair::new().pubkey();
    let results = rpc
        .get_lookup_table_accounts(&[TEST_LOOKUP_TABLE_ADDRESS, random])
        .await?;
    assert_eq!(1, results.len());
    rpc.sync().await;
    assert_eq!(1, rpc.len().await);
    assert_eq!(1, rpc.len_negative().await);
    info!("sleeping");
    sleep(Duration::from_millis(1500)).await;
    rpc.sync().await;
    assert!(rpc.is_empty().await);
    assert!(rpc.is_empty_negative().await);
    let tx: TransactionBuilder = TransactionBuilder::builder()
        .instructions(random_instructions(&kp.pubkey()))
        .build()
        .with_lookup_keys([TEST_LOOKUP_TABLE_ADDRESS, random])
        .with_memo(MEMO_PKG, &[&kp.pubkey()])
        .with_priority_fees(
            &kp.pubkey(),
            &rpc,
            &[solana_system_interface::program::ID],
            1_000_000,
            None,
        )
        .await?;
    let sig = tx.send(&rpc, &kp.pubkey(), &[&kp]).await?;
    info!("{sig}");
    Ok(())
}

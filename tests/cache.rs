mod common;
use {
    common::*,
    futures::future::try_join_all,
    moka::future::Cache,
    solana_keypair::Keypair,
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_rpc_client_api::config::RpcSimulateTransactionConfig,
    solana_signer::Signer,
    soly::{
        BlockHashCacheProvider,
        CounterRpcProvider,
        LookupTableCacheProvider,
        SimpleCacheTransactionProvider,
        TransactionBuilder,
        TransactionRpcProvider,
    },
    std::{sync::Arc, time::Duration},
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
    assert_eq!(1, rpc.len());
    assert_eq!(1, rpc.len_negative());
    info!("sleeping");
    sleep(Duration::from_millis(1500)).await;
    rpc.sync().await;
    assert!(rpc.is_empty());
    assert!(rpc.is_empty_negative());
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

#[tokio::test(flavor = "multi_thread", worker_threads = 5)]
async fn test_simple_cache() -> anyhow::Result<()> {
    let (kp, rpc) = init()?;
    let pubkeys = [kp.pubkey(), kp.pubkey(), kp.pubkey()];
    //    let keys = [kp1, kp2, kp3];
    let span = info_span!("test_simple_cache");
    let _guard = span.enter();
    info!("starting test");
    let counter_rpc = CounterRpcProvider::new(rpc);
    let lookup_rpc = LookupTableCacheProvider::builder()
        .inner(counter_rpc.clone())
        .lookup_cache(
            Cache::builder()
                .time_to_live(Duration::from_secs(60))
                .build(),
        )
        .negative_cache(
            Cache::builder()
                .time_to_live(Duration::from_secs(60))
                .build(),
        )
        .build();
    let blockhash_rpc = BlockHashCacheProvider::new(counter_rpc.clone(), Duration::from_secs(20));
    let rpc = SimpleCacheTransactionProvider::builder()
        .inner(counter_rpc.clone())
        .blockhash_cache(Arc::new(blockhash_rpc))
        .lookup_cache(Arc::new(lookup_rpc))
        .build();
    let transactions: Vec<TransactionBuilder> = vec![
        TransactionBuilder::builder()
            .instructions(random_instructions(&pubkeys[0]))
            .build()
            .with_lookup_keys([TEST_LOOKUP_TABLE_ADDRESS])
            .with_memo(MEMO_PKG, &[&pubkeys[0]])
            .with_priority_fees(
                &pubkeys[0],
                &rpc,
                &[solana_system_interface::program::ID],
                1_000_000,
                None,
            )
            .await?,
        TransactionBuilder::builder()
            .instructions(random_instructions(&pubkeys[1]))
            .build()
            .with_lookup_keys([TEST_LOOKUP_TABLE_ADDRESS])
            .with_memo(MEMO_PKG, &[&pubkeys[1]]),
        TransactionBuilder::builder()
            .instructions(random_instructions(&pubkeys[2]))
            .build()
            .with_lookup_keys([TEST_LOOKUP_TABLE_ADDRESS])
            .push(random_instructions(&pubkeys[2])[0].clone())
            .with_memo(MEMO_PKG, &[&pubkeys[2]]),
    ];

    let futures = transactions
        .iter()
        .enumerate()
        .map(|(i, tx)| {
            let cfg = RpcSimulateTransactionConfig {
                sig_verify: false,
                ..Default::default()
            };
            let span = info_span!("tx", id = %i);
            let r = rpc.clone();
            let pk = pubkeys[i];
            async move {
                let _guard = span.enter();
                // see start of function, this would never be called if it failed, anyways i
                // have to do this because keypair is not send
                let kp = load_keypair().unwrap();
                tx.simulate(&pk, &[&kp], &r, cfg).await
            }
        })
        .collect::<Vec<_>>();
    let _ = try_join_all(futures).await?;
    info!("{}", counter_rpc);
    assert_eq!(1, counter_rpc.get_counter(&soly::RpcMethod::Blockhash));
    assert_eq!(1, counter_rpc.get_counter(&soly::RpcMethod::Fees));
    assert_eq!(1, counter_rpc.get_counter(&soly::RpcMethod::Lookup));
    assert_eq!(4, counter_rpc.get_counter(&soly::RpcMethod::Simulate));

    let tx: TransactionBuilder = spl_memo::build_memo(MEMO_PKG.as_bytes(), &[&kp.pubkey()]).into();
    let sig = tx.send(&rpc, &kp.pubkey(), &[&kp]).await;
    info!(sig = ?sig);
    accept_rpc_client_ref(&rpc);
    Ok(())
}

fn accept_rpc_client_ref<T: AsRef<RpcClient>>(rpc: &T) {
    let _rpc: &RpcClient = rpc.as_ref(); // coverage
}

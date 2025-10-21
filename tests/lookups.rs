mod common;
use {
    common::*,
    solana_message::AddressLookupTableAccount,
    solana_pubkey::{Pubkey, pubkey},
    solana_signer::Signer,
    soly::{TransactionBuilder, fetch_lookup_tables},
    tracing::{info, info_span},
};
const NOT_INITIALIZED: Pubkey = pubkey!("3W6YcoQyFcrSo6K9vixhM2Cfvtjv4KeKSH1FaEKJF1Ug");
const INITIALIZED: Pubkey = pubkey!("FNK9gB5E3cntDRiy3LHwtwQC6qhbVgdBLBMqjRZLEYiK");
const EXPECTED_TABLE: [Pubkey; 3] = [
    pubkey!("So11111111111111111111111111111111111111112"),
    pubkey!("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"),
    pubkey!("8m3uKEn4fMPNVr7nv6RmQYktT4zRqEZzhuZDpG8hQZT4"),
];

const TEST_LOOKUP_TABLE_STATE: [Pubkey; 6] = [
    pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr"),
    pubkey!("11111111111111111111111111111111"),
    pubkey!("ComputeBudget111111111111111111111111111111"),
    pubkey!("AddressLookupTab1e1111111111111111111111111"),
    pubkey!("8X35rQUK2u9hfn8rMPwwr6ZSEUhbmfDPEapp589XyoM1"),
    pubkey!("215r9xfTFVYcE9g3fAUGowauM84egyUvFCbSo3LKNaep"),
];
#[tokio::test]
async fn test_lookup_table() -> anyhow::Result<()> {
    let (_, rpc) = init()?;
    let span = tracing::info_span!("fetch_lookup_tables");
    let _g = span.enter();
    let result = fetch_lookup_tables(&[NOT_INITIALIZED], &rpc).await?;
    assert!(result.is_empty());

    let result = fetch_lookup_tables(&[INITIALIZED], &rpc).await?;
    assert_eq!(1, result.len());
    assert_eq!(result[0].key, INITIALIZED);
    assert_eq!(result[0].addresses, EXPECTED_TABLE);

    let result = fetch_lookup_tables(&[NOT_INITIALIZED, INITIALIZED], &rpc).await?;
    assert_eq!(1, result.len());
    assert_eq!(result[0].key, INITIALIZED);
    assert_eq!(result[0].addresses, EXPECTED_TABLE);

    let result = fetch_lookup_tables(&[], &rpc).await?;
    assert!(result.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_builder_lookup_tables_keys() -> anyhow::Result<()> {
    let (kp, rpc) = init()?;
    let span = tracing::info_span!("builder_lookup_tables_keys");
    let _g = span.enter();
    let payer = kp.pubkey();
    let sig = TransactionBuilder::builder()
        .instructions(random_instructions(&payer))
        .lookup_tables_keys(vec![TEST_LOOKUP_TABLE_ADDRESS])
        .build()
        .with_memo(MEMO_PKG, &[&payer])
        .send(&rpc, &payer, &[&kp])
        .await?;
    info!(sig =? sig);
    Ok(())
}

#[tokio::test]
async fn test_builder_address_lookup_tables_tx() -> anyhow::Result<()> {
    let (kp, rpc) = init()?;
    let span = info_span!("builder_address_lookup_tables");
    let _g = span.enter();
    let payer = kp.pubkey();
    let sig = TransactionBuilder::builder()
        .instructions(random_instructions(&payer))
        .address_lookup_tables(vec![AddressLookupTableAccount {
            key: TEST_LOOKUP_TABLE_ADDRESS,
            addresses: TEST_LOOKUP_TABLE_STATE.to_vec(),
        }])
        .build()
        .with_memo(MEMO_PKG, &[&payer])
        .send(&rpc, &payer, &[&kp])
        .await?;
    info!(sig =? sig);
    Ok(())
}

#[tokio::test]
async fn test_with_address_lookup_tables_tx() -> anyhow::Result<()> {
    let (kp, rpc) = init()?;
    let span = info_span!("with_address_lookup_tables");
    let _g = span.enter();
    let payer = kp.pubkey();
    let sig = TransactionBuilder::builder()
        .instructions(random_instructions(&payer))
        .build()
        .with_address_tables(vec![AddressLookupTableAccount {
            key: TEST_LOOKUP_TABLE_ADDRESS,
            addresses: TEST_LOOKUP_TABLE_STATE.to_vec(),
        }])
        .with_memo(MEMO_PKG, &[&payer])
        .send(&rpc, &payer, &[&kp])
        .await?;
    info!(sig =? sig);
    Ok(())
}

#[tokio::test]
async fn test_with_lookup_keys_tx() -> anyhow::Result<()> {
    let (kp, rpc) = init()?;
    let span = info_span!("with_lookup_keys");
    let _g = span.enter();
    let payer = kp.pubkey();
    let sig = TransactionBuilder::builder()
        .instructions(random_instructions(&payer))
        .build()
        .with_lookup_keys(vec![TEST_LOOKUP_TABLE_ADDRESS])
        .with_memo(MEMO_PKG, &[&payer])
        .send(&rpc, &payer, &[&kp])
        .await?;
    info!(sig =? sig);
    Ok(())
}

#![allow(dead_code)]
use {
    solana_commitment_config::CommitmentConfig,
    solana_keypair::Keypair,
    solana_pubkey::{Pubkey, pubkey},
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_signer::Signer,
    soly::TraceRpcTransactionProvider,
    std::{
        env,
        sync::{Arc, Once},
    },
    tracing::trace,
    tracing_subscriber::{EnvFilter, fmt::format::FmtSpan},
};

pub const MEMO_PKG: &str = "github.com/carteraMesh/soly";
pub static INIT: Once = Once::new();
pub const TEST_LOOKUP_TABLE_ADDRESS: Pubkey =
    pubkey!("njdSrqZgR1gZhLvGoX6wzhSioAczdN669SVt3nktiJe");
pub const RANDO: Pubkey = pubkey!("8X35rQUK2u9hfn8rMPwwr6ZSEUhbmfDPEapp589XyoM1");
pub fn random_instructions(payer: &Pubkey) -> Vec<solana_instruction::Instruction> {
    vec![
        solana_system_interface::instruction::transfer(payer, &RANDO, 1),
        solana_system_interface::instruction::transfer(payer, &RANDO, 2),
        solana_system_interface::instruction::transfer(payer, &RANDO, 3),
    ]
}
#[allow(clippy::unwrap_used, clippy::missing_panics_doc)]
pub fn setup() {
    INIT.call_once(|| {
        if env::var("CI").is_err() {
            // only load .env if not in CI
            if dotenvy::dotenv_override().is_err() {
                eprintln!("no .env file");
            }
        }
        tracing_subscriber::fmt()
            .with_target(true)
            .with_level(true)
            .with_span_events(FmtSpan::CLOSE)
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    });
}

pub fn init() -> anyhow::Result<(Keypair, TraceRpcTransactionProvider)> {
    setup();
    let owner = load_keypair()?;
    let rpc: TraceRpcTransactionProvider = Arc::new(rpc_native()?).into();
    Ok((owner, rpc))
}

#[allow(clippy::expect_fun_call)]
pub fn rpc_native() -> anyhow::Result<RpcClient> {
    let url = env::var("RPC_URL").expect("RPC_URL is not set");
    trace!("using RPC {url}");
    Ok(RpcClient::new_with_commitment(
        url,
        CommitmentConfig::finalized(),
    ))
}

#[allow(clippy::expect_fun_call)]
pub fn load_keypair() -> anyhow::Result<Keypair> {
    let kp_file = env::var("KEYPAIR_FILE").ok();
    let owner = if let Some(kp) = kp_file {
        solana_keypair::read_keypair_file(&kp).expect(&format!("unable to load keypair file {kp}"))
    } else {
        let kp = env::var("TEST_PRIVATE_KEY").expect("TEST_PRIVATE_KEY is not set");
        Keypair::from_base58_string(&kp)
    };
    trace!("using solana address {}", owner.pubkey());
    Ok(owner)
}

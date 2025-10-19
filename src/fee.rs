#[cfg(not(feature = "blocking"))]
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
#[cfg(feature = "blocking")]
use solana_rpc_client::rpc_client::RpcClient;
use {
    super::{Error, Result, TransactionBuilder},
    solana_compute_budget_interface::ComputeBudgetInstruction,
    solana_pubkey::Pubkey,
    solana_rpc_client_api::{config::RpcSimulateTransactionConfig, response::RpcPrioritizationFee},
};

const SOLANA_MAX_COMPUTE_UNITS: u32 = 1_400_000;

impl TransactionBuilder {
    /// Add ComputeBudget instructions to beginning of the transaction. Fails if
    /// ComputeBudget instructions are already present.
    ///
    ///
    /// Use [`TransactionBuilder::unsigned_tx`] to get a transaction for
    /// fee simulation.
    pub fn prepend_compute_budget_instructions(
        mut self,
        units: u32,
        priority_fees: u64,
    ) -> Result<Self> {
        if self
            .instructions
            .iter()
            .any(|ix| ix.program_id == solana_compute_budget_interface::ID)
        {
            return Err(crate::Error::InvalidComputeUnits(
                units.into(),
                "computes is about max solana compute units".to_owned(),
            ));
        }

        self.instructions.splice(0..0, vec![
            ComputeBudgetInstruction::set_compute_unit_limit(units),
            ComputeBudgetInstruction::set_compute_unit_price(priority_fees),
        ]);
        Ok(self)
    }

    fn calc_fees(
        mut self,
        fees: Vec<RpcPrioritizationFee>,
        compute_unit_limit: u32,
        max_prioritization_fee: Option<u64>,
        percentile: Option<u8>,
    ) -> Self {
        if fees.is_empty() {
            tracing::warn!("no RpcPrioritizationFee to calculate fees");
            return self;
        }

        let percentile = percentile.unwrap_or(75).min(100);
        let mut sorted_fees: Vec<u64> = fees.iter().map(|f| f.prioritization_fee).collect();
        sorted_fees.sort();

        let index = (sorted_fees.len() * percentile as usize).saturating_sub(1) / 100;
        let priority_fee = max_prioritization_fee
            .map(|max| sorted_fees[index].min(max))
            .unwrap_or(sorted_fees[index]);

        // Add buffer but cap at Solana's maximum
        let buffered_limit = compute_unit_limit
            .saturating_add(compute_unit_limit / 10)
            .min(SOLANA_MAX_COMPUTE_UNITS);
        // Prepend compute budget instructions to the main instructions
        let compute_budget_instructions = vec![
            ComputeBudgetInstruction::set_compute_unit_limit(buffered_limit),
            ComputeBudgetInstruction::set_compute_unit_price(priority_fee),
        ];

        self.instructions.splice(0..0, compute_budget_instructions);
        self
    }
}

#[cfg(not(feature = "blocking"))]
impl TransactionBuilder {
    pub async fn get_recent_prioritization_fees(
        rpc: &RpcClient,
        accounts: &[Pubkey],
    ) -> Result<Vec<RpcPrioritizationFee>> {
        rpc.get_recent_prioritization_fees(accounts)
            .await
            .map_err(|e| {
                Error::SolanaRpcError(format!("failed to get_recent_prioritization_fees: {e}"))
            })
    }

    /// Quick and dirty fee estimation using recent prioritization fees.
    ///
    /// This convenience method fetches recent prioritization fees and
    /// automatically adds ComputeBudget instructions to the beginning of
    /// your transaction.
    ///
    /// **NOTE** use a real RPC Fee Service if you want more accurate fee
    /// estimation.  This method is for convenience and may not be suitable
    /// for all use cases.
    ///
    /// # Arguments
    /// * `rpc` - RPC client for fetching recent fees
    /// * `max_prioritization_fee` - Optional cap on prioritization fee
    ///   (microlamports per CU)
    /// * `accounts` - Write-locked account addresses to query for relevant
    ///   prioritization fees. Fees are filtered to transactions that interact
    ///   with these accounts. Use program IDs and frequently-accessed accounts
    ///   for best results.
    /// * `percentile` - Fee percentile to use (default: 75th percentile)
    ///
    /// # Example
    /// ```no_run
    /// # use soly::TransactionBuilder;
    /// # use solana_pubkey::Pubkey;
    /// # async fn example(builder: TransactionBuilder, payer: Pubkey, rpc: solana_rpc_client::nonblocking::rpc_client::RpcClient) -> Result<(), Box<dyn std::error::Error>> {
    /// let tx = builder
    ///     .with_priority_fees(
    ///         &payer,
    ///         &rpc,
    ///         Some(5_000_000), // Cap at 5M microlamports/CU
    ///         &[solana_system_interface::program::ID],
    ///         Some(50), // Use 50th percentile (median)
    ///     )
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    /// # Special Considerations
    /// If you use priority fees with a Durable Nonce Transaction, you must
    /// ensure the AdvanceNonce instruction is your transaction's first
    /// instruction. This is critical to ensure your transaction is
    /// successful; otherwise, it will fail.
    ///
    ///
    ///
    /// Reference: <https://solana.com/developers/guides/advanced/how-to-use-priority-fees>
    #[tracing::instrument(skip(rpc, payer, accounts), level = tracing::Level::DEBUG)]
    pub async fn with_priority_fees(
        self,
        payer: &Pubkey,
        rpc: &RpcClient,
        accounts: &[Pubkey],
        max_prioritization_fee: Option<u64>,
        percentile: Option<u8>,
    ) -> Result<Self> {
        if self.instructions.is_empty() {
            return Err(crate::Error::NoInstructions);
        }
        if self
            .instructions
            .iter()
            .any(|ix| ix.program_id == solana_compute_budget_interface::ID)
        {
            tracing::warn!("ComputeBudgetProgram already exists");
            return Ok(self);
        }
        let fees = TransactionBuilder::get_recent_prioritization_fees(rpc, accounts).await?;
        let tx = self.unsigned_tx(payer, rpc).await?;
        let sim_result = self
            .simulate_internal(rpc, &tx, RpcSimulateTransactionConfig {
                sig_verify: false,
                ..Default::default()
            })
            .await?;

        let units = sim_result
            .units_consumed
            .ok_or(crate::Error::InvalidComputeUnits(
                0,
                "RPC returned invalid units".to_string(),
            ))?;
        Ok(self.calc_fees(fees, units.try_into()?, max_prioritization_fee, percentile))
    }
}

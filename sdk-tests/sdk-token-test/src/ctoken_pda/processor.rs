use anchor_lang::prelude::*;
use light_compressed_token_sdk::ValidityProof;

use super::{create_pda::process_create_escrow_pda, mint::process_mint_action, CTokenPda};
use crate::ChainedCtokenInstructionData;

#[allow(dead_code)]
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct PdaCreationData {
    pub amount: u64,
    pub address: [u8; 32],
    pub proof: ValidityProof,
}
// TODO: remove mint to compressed
// TODO: create a second ix which switches the cpis.
use light_sdk::cpi::v2::CpiAccounts;
use light_sdk_types::cpi_accounts::CpiAccountsConfig;
pub fn process_ctoken_pda<'info>(
    ctx: Context<'_, '_, '_, 'info, CTokenPda<'info>>,
    input: ChainedCtokenInstructionData,
) -> Result<()> {
    let config = CpiAccountsConfig {
        cpi_signer: crate::LIGHT_CPI_SIGNER,
        cpi_context: true,
        sol_pool_pda: false,
        sol_compression_recipient: false,
    };

    let cpi_accounts =
        CpiAccounts::new_with_config(ctx.accounts.payer.as_ref(), ctx.remaining_accounts, config);

    process_mint_action(&ctx, &input, &cpi_accounts)?;

    process_create_escrow_pda(
        input.pda_creation.proof,
        input.output_tree_index,
        input.pda_creation.amount,
        input.pda_creation.address,
        input.new_address_params,
        cpi_accounts,
    )?;

    Ok(())
}

use anchor_lang::prelude::*;

declare_id!("Dcc5vv836DzrDrvY8rB7TZD2bhtfXLMCWCHdoJCtQnxy");

#[program]
pub mod test_batch_transactions {
    use anchor_lang::solana_program::poseidon;

    use super::*;

    pub fn initialize(ctx: Context<Initialize>, num_hashes: u64, unique_id: u64) -> Result<()> {
        msg!("id: {}", unique_id);
        // let mut burn_cu_vec = [1; 4000];
        for i in 0..num_hashes {
            poseidon::hash(
                poseidon::Parameters::Bn254X5,
                poseidon::Endianness::LittleEndian,
                &i.to_le_bytes(),
            )
            .unwrap();
        }
        ctx.accounts.counter.counter += 1;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(init_if_needed, seeds =[b"counter"], bump,  payer = signer, space = 8 + 8)]
    /// CHECK:
    pub counter: Account<'info, TransactionCounter>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct TransactionCounter {
    pub counter: u64,
}

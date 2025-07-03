// Example: Proper Config Implementation in Anchor
// This file shows how to implement secure config creation following Solana best practices

use anchor_lang::prelude::*;
use light_sdk::compressible::{create_config, update_compression_config, CompressibleConfig};

#[program]
pub mod example_program {
    use super::*;

    /// Initialize config - only callable by program upgrade authority
    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        compression_delay: u32,
        rent_recipient: Pubkey,
        address_space: Pubkey,
        config_update_authority: Pubkey, // Can be different from program upgrade authority
    ) -> Result<()> {
        // The SDK's create_config validates that the signer is the program's upgrade authority
        create_compression_config_checked(
            &ctx.accounts.config.to_account_info(),
            &ctx.accounts.authority.to_account_info(), // Must be upgrade authority
            &ctx.accounts.program_data.to_account_info(),
            &rent_recipient,
            &address_space,
            compression_delay,
            &ctx.accounts.payer.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            &ctx.program_id,
        )?;

        // If you want the config update authority to be different from the program upgrade authority,
        // you can update it here
        if config_update_authority != ctx.accounts.authority.key() {
            update_compression_config(
                &ctx.accounts.config.to_account_info(),
                &ctx.accounts.authority.to_account_info(),
                Some(&config_update_authority),
                None,
                None,
                None,
            )?;
        }

        Ok(())
    }

    /// Update config - only callable by config's update authority
    pub fn update_config_settings(
        ctx: Context<UpdateConfig>,
        new_compression_delay: Option<u32>,
        new_rent_recipient: Option<Pubkey>,
        new_address_space: Option<Pubkey>,
        new_update_authority: Option<Pubkey>,
    ) -> Result<()> {
        update_compression_config(
            &ctx.accounts.config.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            new_update_authority.as_ref(),
            new_rent_recipient.as_ref(),
            new_address_space.as_ref(),
            new_compression_delay,
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The config PDA to be created
    #[account(
        init,
        payer = payer,
        space = 8 + CompressibleConfig::LEN,
        seeds = [b"compressible_config"],
        bump
    )]
    pub config: AccountInfo<'info>,

    /// The authority that will be able to update config after creation
    pub config_update_authority: AccountInfo<'info>,

    /// The program being configured
    #[account(
        constraint = program.programdata_address()? == Some(program_data.key())
    )]
    pub program: Program<'info, crate::program::ExampleProgram>,

    /// The program's data account
    #[account(
        constraint = program_data.upgrade_authority_address == Some(authority.key())
    )]
    pub program_data: Account<'info, ProgramData>,

    /// The program's upgrade authority (must sign)
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(
        mut,
        seeds = [b"compressible_config"],
        bump,
        // This constraint could also load and check the config's update_authority
        // but the SDK's update_compression_config function will do that check
    )]
    pub config: AccountInfo<'info>,

    /// Must match the update authority stored in config
    pub authority: Signer<'info>,
}

// Alternative: Using has_one constraint if you deserialize the config
#[derive(Accounts)]
pub struct UpdateConfigWithHasOne<'info> {
    #[account(
        mut,
        seeds = [b"compressible_config"],
        bump,
        has_one = update_authority
    )]
    pub config: Account<'info, CompressibleConfig>,

    pub update_authority: Signer<'info>,
}

// Example of using the config in other instructions
#[derive(Accounts)]
pub struct UseConfig<'info> {
    #[account(seeds = [b"compressible_config"], bump)]
    pub config: Account<'info, CompressibleConfig>,
    // Other accounts that use config values...
}

/*
DEPLOYMENT BEST PRACTICES:

1. Deploy your program
2. IMMEDIATELY create the config using the program's upgrade authority
3. Optionally transfer config update authority to a multisig or DAO
4. Monitor the config for any changes

Example deployment script:
```typescript
// 1. Deploy program
const program = await deployProgram();

// 2. Create config immediately
const [configPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("compressible_config")],
    program.programId
);

const [programDataPda] = PublicKey.findProgramAddressSync(
    [program.programId.toBuffer()],
    BPF_LOADER_UPGRADEABLE_PROGRAM_ID
);

await program.methods
    .initializeConfig(
        100, // compression_delay
        rentRecipient,
        addressSpace,
        configUpdateAuthority // Can be same as upgrade authority or different
    )
    .accounts({
        payer: wallet.publicKey,
        config: configPda,
        configUpdateAuthority: configUpdateAuthority,
        program: program.programId,
        programData: programDataPda,
        authority: upgradeAuthority, // Must be the program's upgrade authority
        systemProgram: SystemProgram.programId,
    })
    .signers([upgradeAuthority])
    .rpc();
```
*/

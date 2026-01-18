use borsh::BorshSerialize;
use light_token_interface::instructions::{
    create_token_account::CreateTokenAccountInstructionData,
    extensions::{CompressToPubkey, CompressibleExtensionInstructionData},
};
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::token::{compressible::CompressibleParamsCpi, CompressibleParams};

/// # Create a create ctoken account instruction:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token_sdk::token::CreateTokenAccount;
/// # let payer = Pubkey::new_unique();
/// # let account = Pubkey::new_unique();
/// # let mint = Pubkey::new_unique();
/// # let owner = Pubkey::new_unique();
/// let instruction =
///     CreateTokenAccount::new(payer, account, mint, owner)
///     .instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
#[derive(Debug, Clone)]
pub struct CreateTokenAccount {
    pub payer: Pubkey,
    pub account: Pubkey,
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub compressible: CompressibleParams,
}

impl CreateTokenAccount {
    pub fn new(payer: Pubkey, account: Pubkey, mint: Pubkey, owner: Pubkey) -> Self {
        Self {
            payer,
            account,
            mint,
            owner,
            compressible: CompressibleParams::default(),
        }
    }

    pub fn with_compressible(mut self, compressible: CompressibleParams) -> Self {
        self.compressible = compressible;
        self
    }

    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        let instruction_data = CreateTokenAccountInstructionData {
            owner: light_compressed_account::Pubkey::from(self.owner.to_bytes()),
            compressible_config: Some(CompressibleExtensionInstructionData {
                token_account_version: self.compressible.token_account_version as u8,
                rent_payment: self.compressible.pre_pay_num_epochs,
                compression_only: self.compressible.compression_only as u8,
                write_top_up: self.compressible.lamports_per_write.unwrap_or(0),
                compress_to_account_pubkey: self.compressible.compress_to_account_pubkey,
            }),
        };

        let mut data = Vec::new();
        data.push(18u8); // InitializeAccount3 opcode
        instruction_data
            .serialize(&mut data)
            .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

        let accounts = vec![
            AccountMeta::new(self.account, true),
            AccountMeta::new_readonly(self.mint, false),
            AccountMeta::new(self.payer, true),
            AccountMeta::new_readonly(self.compressible.compressible_config, false),
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program
            AccountMeta::new(self.compressible.rent_sponsor, false),
        ];

        Ok(Instruction {
            program_id: Pubkey::from(light_token_interface::LIGHT_TOKEN_PROGRAM_ID),
            accounts,
            data,
        })
    }
}

/// CPI builder for creating CToken accounts (vaults).
///
/// # Example - Rent-free vault with PDA signing
/// ```rust,ignore
/// CreateTokenAccountCpi {
///     payer: ctx.accounts.payer.to_account_info(),
///     account: ctx.accounts.vault.to_account_info(),
///     mint: ctx.accounts.mint.to_account_info(),
///     owner: ctx.accounts.vault_authority.key(),
/// }
/// .rent_free(
///     ctx.accounts.ctoken_config.to_account_info(),
///     ctx.accounts.rent_sponsor.to_account_info(),
///     ctx.accounts.system_program.to_account_info(),
///     &crate::ID,
/// )
/// .invoke_signed(&[b"vault", mint.key().as_ref(), &[bump]])?;
/// ```
pub struct CreateTokenAccountCpi<'info> {
    pub payer: AccountInfo<'info>,
    pub account: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub owner: Pubkey,
}

impl<'info> CreateTokenAccountCpi<'info> {
    /// Enable rent-free mode with compressible config.
    ///
    /// Returns a builder that can call `.invoke()` or `.invoke_signed(seeds)`.
    /// When using `invoke_signed`, the seeds are used for both PDA signing
    /// and deriving the compress_to address.
    pub fn rent_free(
        self,
        config: AccountInfo<'info>,
        sponsor: AccountInfo<'info>,
        system_program: AccountInfo<'info>,
        program_id: &Pubkey,
    ) -> CreateTokenAccountRentFreeCpi<'info> {
        CreateTokenAccountRentFreeCpi {
            base: self,
            config,
            sponsor,
            system_program,
            program_id: *program_id,
        }
    }

    /// Invoke without rent-free (requires manually constructed compressible params).
    pub fn invoke_with(
        self,
        compressible: CompressibleParamsCpi<'info>,
    ) -> Result<(), ProgramError> {
        LegacyCreateTokenAccountCpi {
            payer: self.payer,
            account: self.account,
            mint: self.mint,
            owner: self.owner,
            compressible,
        }
        .invoke()
    }

    /// Invoke with signing, without rent-free (requires manually constructed compressible params).
    pub fn invoke_signed_with(
        self,
        compressible: CompressibleParamsCpi<'info>,
        signer_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        LegacyCreateTokenAccountCpi {
            payer: self.payer,
            account: self.account,
            mint: self.mint,
            owner: self.owner,
            compressible,
        }
        .invoke_signed(signer_seeds)
    }
}

/// Rent-free enabled CToken account creation CPI.
pub struct CreateTokenAccountRentFreeCpi<'info> {
    base: CreateTokenAccountCpi<'info>,
    config: AccountInfo<'info>,
    sponsor: AccountInfo<'info>,
    system_program: AccountInfo<'info>,
    program_id: Pubkey,
}

impl<'info> CreateTokenAccountRentFreeCpi<'info> {
    /// Invoke CPI for non-program-owned accounts.
    pub fn invoke(self) -> Result<(), ProgramError> {
        let defaults = CompressibleParams::default();

        let cpi = LegacyCreateTokenAccountCpi {
            payer: self.base.payer,
            account: self.base.account,
            mint: self.base.mint,
            owner: self.base.owner,
            compressible: CompressibleParamsCpi {
                compressible_config: self.config,
                rent_sponsor: self.sponsor,
                system_program: self.system_program,
                pre_pay_num_epochs: defaults.pre_pay_num_epochs,
                lamports_per_write: defaults.lamports_per_write,
                compress_to_account_pubkey: None,
                token_account_version: defaults.token_account_version,
                compression_only: defaults.compression_only,
            },
        };
        cpi.invoke()
    }

    /// Invoke CPI with PDA signing for program-owned accounts.
    ///
    /// Seeds are used for both signing AND deriving the compress_to address.
    pub fn invoke_signed(self, seeds: &[&[u8]]) -> Result<(), ProgramError> {
        let defaults = CompressibleParams::default();

        // Build CompressToPubkey from signer seeds
        let bump = seeds.last().and_then(|s| s.first()).copied().unwrap_or(0);

        let seed_vecs: Vec<Vec<u8>> = seeds
            .iter()
            .take(seeds.len().saturating_sub(1))
            .map(|s| s.to_vec())
            .collect();

        let compress_to = CompressToPubkey {
            bump,
            program_id: self.program_id.to_bytes(),
            seeds: seed_vecs,
        };

        let cpi = LegacyCreateTokenAccountCpi {
            payer: self.base.payer,
            account: self.base.account,
            mint: self.base.mint,
            owner: self.base.owner,
            compressible: CompressibleParamsCpi {
                compressible_config: self.config,
                rent_sponsor: self.sponsor,
                system_program: self.system_program,
                pre_pay_num_epochs: defaults.pre_pay_num_epochs,
                lamports_per_write: defaults.lamports_per_write,
                compress_to_account_pubkey: Some(compress_to),
                token_account_version: defaults.token_account_version,
                compression_only: defaults.compression_only,
            },
        };
        cpi.invoke_signed(&[seeds])
    }
}

/// Internal legacy CPI struct with full compressible params.
struct LegacyCreateTokenAccountCpi<'info> {
    payer: AccountInfo<'info>,
    account: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    owner: Pubkey,
    compressible: CompressibleParamsCpi<'info>,
}

impl<'info> LegacyCreateTokenAccountCpi<'info> {
    fn instruction(&self) -> Result<Instruction, ProgramError> {
        CreateTokenAccount {
            payer: *self.payer.key,
            account: *self.account.key,
            mint: *self.mint.key,
            owner: self.owner,
            compressible: CompressibleParams {
                compressible_config: *self.compressible.compressible_config.key,
                rent_sponsor: *self.compressible.rent_sponsor.key,
                pre_pay_num_epochs: self.compressible.pre_pay_num_epochs,
                lamports_per_write: self.compressible.lamports_per_write,
                compress_to_account_pubkey: self.compressible.compress_to_account_pubkey.clone(),
                token_account_version: self.compressible.token_account_version,
                compression_only: self.compressible.compression_only,
            },
        }
        .instruction()
    }

    fn invoke(self) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        let account_infos = [
            self.account,
            self.mint,
            self.payer,
            self.compressible.compressible_config,
            self.compressible.system_program,
            self.compressible.rent_sponsor,
        ];
        invoke(&instruction, &account_infos)
    }

    fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        let account_infos = [
            self.account,
            self.mint,
            self.payer,
            self.compressible.compressible_config,
            self.compressible.system_program,
            self.compressible.rent_sponsor,
        ];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

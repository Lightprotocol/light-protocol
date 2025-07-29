use light_account_checks::AccountInfoTrait;

use crate::error::{LightTokenSdkTypeError, Result};

#[repr(usize)]
pub enum MintToCompressedAccountInfosIndex {
    // Static non-CPI accounts first
    Authority = 0,
    // Optional decompressed accounts (if is_decompressed = true)
    Mint = 1,               // Only present if is_decompressed
    TokenPoolPda = 2,       // Only present if is_decompressed
    TokenProgram = 3,       // Only present if is_decompressed
    LightSystemProgram = 4, // Always present (index adjusted based on decompressed)
    // LightSystemAccounts (7 accounts)
    FeePayer = 5, // (index adjusted based on decompressed)
    CpiAuthorityPda = 6,
    RegisteredProgramPda = 7,
    NoopProgram = 8,
    AccountCompressionAuthority = 9,
    AccountCompressionProgram = 10,
    SystemProgram = 11,
    SelfProgram = 12,
    // Optional sol pool
    SolPoolPda = 13, // Only present if with_lamports
    // UpdateOneCompressedAccountTreeAccounts (3 accounts)
    InMerkleTree = 14, // (index adjusted based on sol_pool_pda)
    InOutputQueue = 15,
    OutOutputQueue = 16,
    // Final account
    TokensOutQueue = 17,
}

pub struct MintToCompressedAccountInfos<'a, T: AccountInfoTrait + Clone> {
    fee_payer: &'a T,
    authority: &'a T,
    accounts: &'a [T],
    config: MintToCompressedAccountInfosConfig,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct MintToCompressedAccountInfosConfig {
    pub is_decompressed: bool, // Whether mint, token_pool_pda, token_program are present
    pub has_sol_pool_pda: bool, // Whether sol_pool_pda is present
}

impl MintToCompressedAccountInfosConfig {
    pub const fn new(is_decompressed: bool, has_sol_pool_pda: bool) -> Self {
        Self {
            is_decompressed,
            has_sol_pool_pda,
        }
    }
}

impl<'a, T: AccountInfoTrait + Clone> MintToCompressedAccountInfos<'a, T> {
    pub fn new(
        fee_payer: &'a T,
        authority: &'a T,
        accounts: &'a [T],
        config: MintToCompressedAccountInfosConfig,
    ) -> Self {
        Self {
            fee_payer,
            authority,
            accounts,
            config,
        }
    }

    pub fn fee_payer(&self) -> &'a T {
        self.fee_payer
    }

    pub fn authority(&self) -> &'a T {
        self.authority
    }

    fn get_adjusted_index(&self, base_index: usize) -> usize {
        let mut adjusted = base_index;

        // Adjust for decompressed accounts (mint, token_pool_pda, token_program are indices 1,2,3)
        // If not decompressed, all indices after LightSystemProgram shift down by 3
        if !self.config.is_decompressed
            && base_index > MintToCompressedAccountInfosIndex::LightSystemProgram as usize
        {
            adjusted -= 3;
        }

        // Adjust for sol_pool_pda (index 13)
        // If no sol_pool_pda, all indices after it shift down by 1
        if !self.config.has_sol_pool_pda
            && base_index > MintToCompressedAccountInfosIndex::SolPoolPda as usize
        {
            adjusted -= 1;
        }

        adjusted
    }

    pub fn mint(&self) -> Result<&'a T> {
        if !self.config.is_decompressed {
            return Err(LightTokenSdkTypeError::MintUndefinedForBatchCompress);
        }
        let index = self.get_adjusted_index(MintToCompressedAccountInfosIndex::Mint as usize);
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn token_pool_pda(&self) -> Result<&'a T> {
        if !self.config.is_decompressed {
            return Err(LightTokenSdkTypeError::TokenPoolUndefinedForCompressed);
        }
        let index =
            self.get_adjusted_index(MintToCompressedAccountInfosIndex::TokenPoolPda as usize);
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn token_program(&self) -> Result<&'a T> {
        if !self.config.is_decompressed {
            return Err(LightTokenSdkTypeError::TokenProgramUndefinedForCompressed);
        }
        let index =
            self.get_adjusted_index(MintToCompressedAccountInfosIndex::TokenProgram as usize);
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn light_system_program(&self) -> Result<&'a T> {
        let index =
            self.get_adjusted_index(MintToCompressedAccountInfosIndex::LightSystemProgram as usize);
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn cpi_authority_pda(&self) -> Result<&'a T> {
        let index =
            self.get_adjusted_index(MintToCompressedAccountInfosIndex::CpiAuthorityPda as usize);
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn registered_program_pda(&self) -> Result<&'a T> {
        let index = self
            .get_adjusted_index(MintToCompressedAccountInfosIndex::RegisteredProgramPda as usize);
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn noop_program(&self) -> Result<&'a T> {
        let index =
            self.get_adjusted_index(MintToCompressedAccountInfosIndex::NoopProgram as usize);
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn account_compression_authority(&self) -> Result<&'a T> {
        let index = self.get_adjusted_index(
            MintToCompressedAccountInfosIndex::AccountCompressionAuthority as usize,
        );
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn account_compression_program(&self) -> Result<&'a T> {
        let index = self.get_adjusted_index(
            MintToCompressedAccountInfosIndex::AccountCompressionProgram as usize,
        );
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn system_program(&self) -> Result<&'a T> {
        let index =
            self.get_adjusted_index(MintToCompressedAccountInfosIndex::SystemProgram as usize);
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn self_program(&self) -> Result<&'a T> {
        let index =
            self.get_adjusted_index(MintToCompressedAccountInfosIndex::SelfProgram as usize);
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn sol_pool_pda(&self) -> Result<&'a T> {
        if !self.config.has_sol_pool_pda {
            return Err(LightTokenSdkTypeError::SolPoolPdaUndefined);
        }
        let index = self.get_adjusted_index(MintToCompressedAccountInfosIndex::SolPoolPda as usize);
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn in_merkle_tree(&self) -> Result<&'a T> {
        let index =
            self.get_adjusted_index(MintToCompressedAccountInfosIndex::InMerkleTree as usize);
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn in_output_queue(&self) -> Result<&'a T> {
        let index =
            self.get_adjusted_index(MintToCompressedAccountInfosIndex::InOutputQueue as usize);
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn out_output_queue(&self) -> Result<&'a T> {
        let index =
            self.get_adjusted_index(MintToCompressedAccountInfosIndex::OutOutputQueue as usize);
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn tokens_out_queue(&self) -> Result<&'a T> {
        let index =
            self.get_adjusted_index(MintToCompressedAccountInfosIndex::TokensOutQueue as usize);
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn get_account_info(&self, index: usize) -> Result<&'a T> {
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn to_account_infos(&self) -> Vec<T> {
        [
            vec![self.fee_payer.clone()],
            vec![self.authority.clone()],
            self.accounts.to_vec(),
        ]
        .concat()
    }

    pub fn account_infos(&self) -> &'a [T] {
        self.accounts
    }

    pub fn config(&self) -> &MintToCompressedAccountInfosConfig {
        &self.config
    }

    pub fn system_accounts_len(&self) -> usize {
        let mut len = 14; // Base accounts: authority(1) + light_system(7) + tree_accounts(3) + tokens_out_queue(1) + 2 signers

        if self.config.is_decompressed {
            len += 3; // mint, token_pool_pda, token_program
        }

        if self.config.has_sol_pool_pda {
            len += 1; // sol_pool_pda
        }

        len
    }
}

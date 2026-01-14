use light_account_checks::AccountInfoTrait;

use crate::error::{LightTokenSdkTypeError, Result};

#[repr(usize)]
pub enum CreateCompressedMintAccountInfosIndex {
    // Static non-CPI accounts first
    MintSigner = 0,
    LightSystemProgram = 1,
    // LightSystemAccounts (7 accounts)
    // FeePayer = 2, this is not ideal, if we put the fee payer in this position we don't have to copy account infos at all.
    CpiAuthorityPda = 2,
    RegisteredProgramPda = 3,
    NoopProgram = 4,
    AccountCompressionAuthority = 5,
    AccountCompressionProgram = 6,
    SystemProgram = 7,
    SelfProgram = 8,
    // CreateCompressedAccountTreeAccounts (2 accounts)
    AddressMerkleTree = 9,
    OutOutputQueue = 10,
}

pub struct CreateCompressedMintAccountInfos<'a, T: AccountInfoTrait + Clone> {
    fee_payer: &'a T,
    accounts: &'a [T],
}

impl<'a, T: AccountInfoTrait + Clone> CreateCompressedMintAccountInfos<'a, T> {
    // Idea new_with_fee_payer and new
    pub fn new(fee_payer: &'a T, accounts: &'a [T]) -> Self {
        Self {
            fee_payer,
            accounts,
        }
    }

    pub fn fee_payer(&self) -> &'a T {
        self.fee_payer
    }

    pub fn mint_signer(&self) -> Result<&'a T> {
        let index = CreateCompressedMintAccountInfosIndex::MintSigner as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn light_system_program(&self) -> Result<&'a T> {
        let index = CreateCompressedMintAccountInfosIndex::LightSystemProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn cpi_authority_pda(&self) -> Result<&'a T> {
        let index = CreateCompressedMintAccountInfosIndex::CpiAuthorityPda as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn registered_program_pda(&self) -> Result<&'a T> {
        let index = CreateCompressedMintAccountInfosIndex::RegisteredProgramPda as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn noop_program(&self) -> Result<&'a T> {
        let index = CreateCompressedMintAccountInfosIndex::NoopProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn account_compression_authority(&self) -> Result<&'a T> {
        let index = CreateCompressedMintAccountInfosIndex::AccountCompressionAuthority as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn account_compression_program(&self) -> Result<&'a T> {
        let index = CreateCompressedMintAccountInfosIndex::AccountCompressionProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn system_program(&self) -> Result<&'a T> {
        let index = CreateCompressedMintAccountInfosIndex::SystemProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn self_program(&self) -> Result<&'a T> {
        let index = CreateCompressedMintAccountInfosIndex::SelfProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn address_merkle_tree(&self) -> Result<&'a T> {
        let index = CreateCompressedMintAccountInfosIndex::AddressMerkleTree as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn out_output_queue(&self) -> Result<&'a T> {
        let index = CreateCompressedMintAccountInfosIndex::OutOutputQueue as usize;
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
        [vec![self.fee_payer.clone()], self.accounts.to_vec()].concat()
    }

    pub fn account_infos(&self) -> &'a [T] {
        self.accounts
    }

    pub fn system_accounts_len(&self) -> usize {
        11 // mint_signer(1) + light_system_program(1) + light_system(7) + tree_accounts(2)
    }

    pub fn tree_pubkeys(&self) -> Result<[T; 2]> {
        Ok([
            self.address_merkle_tree()?.clone(),
            self.out_output_queue()?.clone(),
        ])
    }
}

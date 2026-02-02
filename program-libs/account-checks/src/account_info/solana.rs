use super::{account_info_trait::AccountInfoTrait, account_meta_trait::AccountMetaTrait};
use crate::error::AccountError;

/// Implement trait for solana AccountInfo
impl AccountInfoTrait for solana_account_info::AccountInfo<'_> {
    type Pubkey = solana_pubkey::Pubkey;
    type DataRef<'b>
        = core::cell::Ref<'b, [u8]>
    where
        Self: 'b;
    type DataRefMut<'b>
        = core::cell::RefMut<'b, [u8]>
    where
        Self: 'b;

    fn key(&self) -> [u8; 32] {
        self.key.to_bytes()
    }

    fn key_ref(&self) -> &[u8] {
        self.key.as_ref()
    }

    fn pubkey(&self) -> Self::Pubkey {
        *self.key
    }

    fn pubkey_from_bytes(bytes: [u8; 32]) -> Self::Pubkey {
        solana_pubkey::Pubkey::from(bytes)
    }

    fn is_writable(&self) -> bool {
        self.is_writable
    }

    fn is_signer(&self) -> bool {
        self.is_signer
    }

    fn executable(&self) -> bool {
        self.executable
    }

    fn lamports(&self) -> u64 {
        **self.lamports.borrow()
    }

    fn data_len(&self) -> usize {
        self.data.borrow().len()
    }

    fn try_borrow_data(&self) -> Result<Self::DataRef<'_>, AccountError> {
        self.data
            .try_borrow()
            .map(|r| core::cell::Ref::map(r, |data| &**data))
            .map_err(Into::into)
    }

    fn try_borrow_mut_data(&self) -> Result<Self::DataRefMut<'_>, AccountError> {
        self.data
            .try_borrow_mut()
            .map(|r| core::cell::RefMut::map(r, |data| &mut **data))
            .map_err(Into::into)
    }

    fn is_owned_by(&self, program: &[u8; 32]) -> bool {
        self.owner.as_ref() == program
    }

    fn find_program_address(seeds: &[&[u8]], program_id: &[u8; 32]) -> ([u8; 32], u8) {
        let program_pubkey = solana_pubkey::Pubkey::from(*program_id);
        let (pubkey, bump) = solana_pubkey::Pubkey::find_program_address(seeds, &program_pubkey);
        (pubkey.to_bytes(), bump)
    }

    fn create_program_address(
        seeds: &[&[u8]],
        program_id: &[u8; 32],
    ) -> Result<[u8; 32], AccountError> {
        let program_pubkey = solana_pubkey::Pubkey::from(*program_id);
        solana_pubkey::Pubkey::create_program_address(seeds, &program_pubkey)
            .map(|pubkey| pubkey.to_bytes())
            .map_err(|_| AccountError::InvalidSeeds)
    }

    fn get_min_rent_balance(size: usize) -> Result<u64, AccountError> {
        use solana_sysvar::Sysvar;
        solana_sysvar::rent::Rent::get()
            .map(|rent| rent.minimum_balance(size))
            .map_err(|_| AccountError::FailedBorrowRentSysvar)
    }

    fn get_current_slot() -> Result<u64, AccountError> {
        use solana_sysvar::Sysvar;
        solana_sysvar::clock::Clock::get()
            .map(|c| c.slot)
            .map_err(|_| AccountError::FailedSysvarAccess)
    }

    fn assign(&self, new_owner: &[u8; 32]) -> Result<(), AccountError> {
        self.assign(&solana_pubkey::Pubkey::from(*new_owner));
        Ok(())
    }

    fn realloc(&self, new_len: usize, zero_init: bool) -> Result<(), AccountError> {
        #[allow(deprecated)]
        self.realloc(new_len, zero_init)
            .map_err(|_| AccountError::InvalidAccountSize)
    }

    fn sub_lamports(&self, amount: u64) -> Result<(), AccountError> {
        let mut lamports = self
            .try_borrow_mut_lamports()
            .map_err(|_| AccountError::BorrowAccountDataFailed)?;
        **lamports = lamports
            .checked_sub(amount)
            .ok_or(AccountError::ArithmeticOverflow)?;
        Ok(())
    }

    fn add_lamports(&self, amount: u64) -> Result<(), AccountError> {
        let mut lamports = self
            .try_borrow_mut_lamports()
            .map_err(|_| AccountError::BorrowAccountDataFailed)?;
        **lamports = lamports
            .checked_add(amount)
            .ok_or(AccountError::ArithmeticOverflow)?;
        Ok(())
    }

    fn close(&self, destination: &Self) -> Result<(), AccountError> {
        crate::close_account::close_account(self, destination)
    }

    #[inline(never)]
    fn create_pda_account(
        &self,
        lamports: u64,
        space: u64,
        owner: &[u8; 32],
        pda_seeds: &[&[u8]],
        rent_payer: &Self,
        rent_payer_seeds: &[&[u8]],
        system_program: &Self,
    ) -> Result<(), AccountError> {
        use solana_cpi::invoke_signed;
        use solana_system_interface::instruction as system_instruction;

        let owner_pubkey = solana_pubkey::Pubkey::from(*owner);

        // Cold path: account already has lamports (e.g., attacker donation).
        // CreateAccount would fail, so use Assign + Allocate + Transfer.
        if self.lamports() > 0 {
            return create_pda_with_lamports_solana(
                self,
                lamports,
                space,
                &owner_pubkey,
                pda_seeds,
                rent_payer,
                rent_payer_seeds,
                system_program,
            );
        }

        // Normal path: CreateAccount
        let create_ix = system_instruction::create_account(
            rent_payer.key,
            self.key,
            lamports,
            space,
            &owner_pubkey,
        );
        // Only include rent_payer_seeds when the payer is itself a PDA.
        // Passing empty seeds to invoke_signed causes create_program_address(&[], program_id)
        // which can fail if the result happens to land on the ed25519 curve.
        if rent_payer_seeds.is_empty() {
            invoke_signed(
                &create_ix,
                &[rent_payer.clone(), self.clone()],
                &[pda_seeds],
            )
        } else {
            invoke_signed(
                &create_ix,
                &[rent_payer.clone(), self.clone()],
                &[rent_payer_seeds, pda_seeds],
            )
        }
        .map_err(|_| AccountError::InvalidAccount)
    }

    fn transfer_lamports_cpi(
        &self,
        destination: &Self,
        lamports: u64,
        signer_seeds: &[&[u8]],
    ) -> Result<(), AccountError> {
        use solana_cpi::invoke_signed;
        use solana_system_interface::instruction as system_instruction;

        let ix = system_instruction::transfer(self.key, destination.key, lamports);
        invoke_signed(&ix, &[self.clone(), destination.clone()], &[signer_seeds])
            .map_err(|_| AccountError::InvalidAccount)
    }

    fn invoke_cpi(
        program_id: &[u8; 32],
        instruction_data: &[u8],
        account_metas: &[super::account_info_trait::CpiMeta],
        account_infos: &[Self],
        signer_seeds: &[&[&[u8]]],
    ) -> Result<(), AccountError> {
        use solana_cpi::invoke_signed;

        let metas: Vec<solana_instruction::AccountMeta> = account_metas
            .iter()
            .map(|m| solana_instruction::AccountMeta {
                pubkey: solana_pubkey::Pubkey::from(m.pubkey),
                is_signer: m.is_signer,
                is_writable: m.is_writable,
            })
            .collect();

        let ix = solana_instruction::Instruction {
            program_id: solana_pubkey::Pubkey::from(*program_id),
            accounts: metas,
            data: instruction_data.to_vec(),
        };

        invoke_signed(&ix, account_infos, signer_seeds).map_err(|_| AccountError::InvalidAccount)
    }
}

impl AccountMetaTrait for solana_instruction::AccountMeta {
    type Pubkey = solana_pubkey::Pubkey;

    fn new(pubkey: solana_pubkey::Pubkey, is_signer: bool, is_writable: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable,
        }
    }

    fn pubkey_to_bytes(pubkey: solana_pubkey::Pubkey) -> [u8; 32] {
        pubkey.to_bytes()
    }

    fn pubkey_from_bytes(bytes: [u8; 32]) -> solana_pubkey::Pubkey {
        solana_pubkey::Pubkey::from(bytes)
    }

    fn pubkey_bytes(&self) -> [u8; 32] {
        self.pubkey.to_bytes()
    }

    fn is_signer(&self) -> bool {
        self.is_signer
    }

    fn is_writable(&self) -> bool {
        self.is_writable
    }

    fn set_is_signer(&mut self, val: bool) {
        self.is_signer = val;
    }

    fn set_is_writable(&mut self, val: bool) {
        self.is_writable = val;
    }
}

/// Cold path for create_pda_account when account already has lamports.
#[cold]
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn create_pda_with_lamports_solana<'a>(
    account: &solana_account_info::AccountInfo<'a>,
    lamports: u64,
    space: u64,
    owner: &solana_pubkey::Pubkey,
    pda_seeds: &[&[u8]],
    rent_payer: &solana_account_info::AccountInfo<'a>,
    rent_payer_seeds: &[&[u8]],
    system_program: &solana_account_info::AccountInfo<'a>,
) -> Result<(), AccountError> {
    use solana_cpi::invoke_signed;
    use solana_system_interface::instruction as system_instruction;

    let current_lamports = account.lamports();

    // Assign owner
    let assign_ix = system_instruction::assign(account.key, owner);
    invoke_signed(&assign_ix, std::slice::from_ref(account), &[pda_seeds])
        .map_err(|_| AccountError::InvalidAccount)?;

    // Allocate space
    let allocate_ix = system_instruction::allocate(account.key, space);
    invoke_signed(&allocate_ix, std::slice::from_ref(account), &[pda_seeds])
        .map_err(|_| AccountError::InvalidAccount)?;

    // Transfer remaining lamports for rent-exemption if needed
    if lamports > current_lamports {
        let transfer_ix =
            system_instruction::transfer(rent_payer.key, account.key, lamports - current_lamports);
        // Only include rent_payer_seeds when the payer is itself a PDA.
        if rent_payer_seeds.is_empty() {
            invoke_signed(
                &transfer_ix,
                &[rent_payer.clone(), account.clone(), system_program.clone()],
                &[],
            )
        } else {
            invoke_signed(
                &transfer_ix,
                &[rent_payer.clone(), account.clone(), system_program.clone()],
                &[rent_payer_seeds],
            )
        }
        .map_err(|_| AccountError::InvalidAccount)?;
    }

    Ok(())
}

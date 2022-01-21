use crate::poseidon_merkle_tree::instructions::*;
use crate::poseidon_merkle_tree::instructions_poseidon::{
    permute_instruction_3, permute_instruction_6, permute_instruction_first,
    permute_instruction_last,
};
use crate::poseidon_merkle_tree::state;
use crate::poseidon_merkle_tree::state::{
    HashBytes, InitMerkleTreeBytes, MerkleTree, TwoLeavesBytesPda,
};
use crate::IX_ORDER;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::rent::Rent,
    sysvar::Sysvar,
};

pub struct MerkleTreeProcessor<'a, 'b> {
    merkle_tree_account: Option<&'a AccountInfo<'b>>,
    main_account: Option<&'a AccountInfo<'b>>,
    unpacked_merkle_tree: MerkleTree,
}

impl<'a, 'b> MerkleTreeProcessor<'a, 'b> {
    pub fn new(
        main_account: Option<&'a AccountInfo<'b>>,
        merkle_tree_account: Option<&'a AccountInfo<'b>>,
    ) -> Result<Self, ProgramError> {
        let empty_smt = MerkleTree {
            is_initialized: false,
            levels: 1,
            filled_subtrees: vec![vec![0 as u8; 1]; 1],
            //zeros: vec![vec![0 as u8; 1];1],
            current_root_index: 0,
            next_index: 0,
            root_history_size: 10,
            roots: vec![0 as u8; 1],
            //leaves: vec![0],
            current_total_deposits: 0,
            inserted_leaf: false,
            inserted_root: false,
            pubkey_locked: vec![0],
            time_locked: 0,
        };

        Ok(MerkleTreeProcessor {
            merkle_tree_account: merkle_tree_account,
            main_account: main_account,
            unpacked_merkle_tree: empty_smt,
        })
    }

    pub fn initialize_new_merkle_tree_from_bytes(
        &mut self,
        init_bytes: &[u8],
    ) -> Result<(), ProgramError> {
        let mut unpacked_init_merkle_tree =
            InitMerkleTreeBytes::unpack(&self.merkle_tree_account.unwrap().data.borrow())?;

        for i in 0..unpacked_init_merkle_tree.bytes.len() {
            unpacked_init_merkle_tree.bytes[i] = init_bytes[i];
        }

        InitMerkleTreeBytes::pack_into_slice(
            &unpacked_init_merkle_tree,
            &mut self.merkle_tree_account.unwrap().data.borrow_mut(),
        );
        if unpacked_init_merkle_tree.bytes[0..init_bytes.len()] != init_bytes[..] {
            msg!("merkle tree init failed");
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    pub fn process_instruction(&mut self, accounts: &[AccountInfo]) -> Result<(), ProgramError> {
        let account = &mut accounts.iter();
        let signer = next_account_info(account)?;
        let _main_account = next_account_info(account)?;
        let mut main_account_data = HashBytes::unpack(&self.main_account.unwrap().data.borrow())?;
        msg!(
            "main_account_data.current_instruction_index {}",
            main_account_data.current_instruction_index
        );

        if main_account_data.current_instruction_index < IX_ORDER.len()
            && (IX_ORDER[main_account_data.current_instruction_index] == 14
                || IX_ORDER[main_account_data.current_instruction_index] == 25)
        {
            let merkle_tree_account = next_account_info(account)?;
            let mut merkle_tree_account_data =
                MerkleTree::unpack(&merkle_tree_account.data.borrow())?;

            merkle_tree_pubkey_check(*merkle_tree_account.key)?;
            pubkey_check(
                *signer.key,
                solana_program::pubkey::Pubkey::new(&merkle_tree_account_data.pubkey_locked),
                String::from("merkle tree locked by other account"),
            )?;

            _process_instruction(
                IX_ORDER[main_account_data.current_instruction_index],
                &mut main_account_data,
                &mut merkle_tree_account_data,
            )?;

            MerkleTree::pack_into_slice(
                &merkle_tree_account_data,
                &mut merkle_tree_account.data.borrow_mut(),
            );
        } else if main_account_data.current_instruction_index < IX_ORDER.len()
            && IX_ORDER[main_account_data.current_instruction_index] == 34
        {
            //locks and transfers deposit money
            let merkle_tree_account = next_account_info(account)?;
            let mut merkle_tree_account_data =
                MerkleTree::unpack(&merkle_tree_account.data.borrow())?;
            let current_slot = <Clock as Sysvar>::get()?.slot.clone();
            msg!("Current slot: {:?}", current_slot);

            msg!("locked at slot: {}", merkle_tree_account_data.time_locked);
            msg!(
                "lock ends at slot: {}",
                merkle_tree_account_data.time_locked + 10 // TODO: change lok back to higher no.
            );

            //lock
            if merkle_tree_account_data.time_locked == 0
                || merkle_tree_account_data.time_locked + 10 < current_slot
            {
                merkle_tree_account_data.time_locked = <Clock as Sysvar>::get()?.slot;
                merkle_tree_account_data.pubkey_locked = signer.key.to_bytes().to_vec();
                msg!("locked at {}", merkle_tree_account_data.time_locked);
                msg!("locked by: {:?}", merkle_tree_account_data.pubkey_locked);
                msg!(
                    "locked by: {:?}",
                    solana_program::pubkey::Pubkey::new(&merkle_tree_account_data.pubkey_locked)
                );
            } else if merkle_tree_account_data.time_locked + 10 > current_slot
            /*&& solana_program::pubkey::Pubkey::new(&merkle_tree_account_data.pubkey_locked[..]) != *signer.key*/
            {
                msg!("contract is still locked");
                return Err(ProgramError::InvalidInstructionData);
            } else {
                merkle_tree_account_data.time_locked = <Clock as Sysvar>::get()?.slot;
                merkle_tree_account_data.pubkey_locked = signer.key.to_bytes().to_vec();
            }

            merkle_tree_pubkey_check(*merkle_tree_account.key)?;
            MerkleTree::pack_into_slice(
                &merkle_tree_account_data,
                &mut merkle_tree_account.data.borrow_mut(),
            );
        } else if IX_ORDER[main_account_data.current_instruction_index] == 0
            || IX_ORDER[main_account_data.current_instruction_index] == 1
            || IX_ORDER[main_account_data.current_instruction_index] == 2
            || IX_ORDER[main_account_data.current_instruction_index] == 3
        {
            let merkle_tree_account = next_account_info(account)?;
            merkle_tree_pubkey_check(*merkle_tree_account.key)?;
            //hash instructions do not need the merkle tree
            _process_instruction(
                IX_ORDER[main_account_data.current_instruction_index],
                &mut main_account_data,
                &mut self.unpacked_merkle_tree,
            )?;
        } else if IX_ORDER[main_account_data.current_instruction_index] == 241 {
            //inserting root and creating leave pda accounts
            //the pda account should be created in the same tx, the pda account also functions as escrow account

            msg!(
                "instruction: {}",
                IX_ORDER[main_account_data.current_instruction_index]
            );
            let leaf_pda = next_account_info(account)?;
            msg!("here-2");
            let mut leaf_pda_account_data = TwoLeavesBytesPda::unpack(&leaf_pda.data.borrow())?;
            msg!("here-3");
            let _nullifer0 = next_account_info(account)?;
            let _nullifer1 = next_account_info(account)?;
            let merkle_tree_account = next_account_info(account)?;
            let mut merkle_tree_account_data =
                MerkleTree::unpack(&merkle_tree_account.data.borrow())?;

            //checking if signer locked
            pubkey_check(
                *signer.key,
                solana_program::pubkey::Pubkey::new(&merkle_tree_account_data.pubkey_locked),
                String::from("merkle tree locked by other account"),
            )?;
            //checking merkle tree pubkey for consistency
            merkle_tree_pubkey_check(*merkle_tree_account.key)?;

            //insert root into merkle tree
            insert_last_double(&mut merkle_tree_account_data, &mut main_account_data)?;

            //check leaves account is rent exempt
            let rent = Rent::free();
            if !rent.is_exempt(**leaf_pda.lamports.borrow(), 106) {
                msg!("leaves account is not rent exempt");
                return Err(ProgramError::InvalidAccountData);
            }
            //save leaves into pda account
            leaf_pda_account_data.leaf_left = main_account_data.leaf_left.clone();
            leaf_pda_account_data.leaf_right = main_account_data.leaf_right.clone();
            msg!(
                "here2 main_account_data.current_index {}",
                main_account_data.current_index.clone()
            );
            leaf_pda_account_data.left_leaf_index = merkle_tree_account_data.next_index - 2;
            leaf_pda_account_data.merkle_tree_pubkey =
                state::MERKLE_TREE_ACC_BYTES.to_vec().clone();

            msg!("Lock set at slot {}", merkle_tree_account_data.time_locked);
            msg!("lock released at slot: {}", <Clock as Sysvar>::get()?.slot);
            merkle_tree_account_data.time_locked = 0;
            merkle_tree_account_data.pubkey_locked = vec![0; 32];

            MerkleTree::pack_into_slice(
                &merkle_tree_account_data,
                &mut merkle_tree_account.data.borrow_mut(),
            );
            TwoLeavesBytesPda::pack_into_slice(
                &leaf_pda_account_data,
                &mut leaf_pda.data.borrow_mut(),
            );
        }
        main_account_data.current_instruction_index += 1;
        HashBytes::pack_into_slice(
            &main_account_data,
            &mut self.main_account.unwrap().data.borrow_mut(),
        );

        Ok(())
    }
}

pub fn _process_instruction(
    id: u8,
    main_account_data: &mut HashBytes,
    merkle_tree_account_data: &mut MerkleTree,
) -> Result<(), ProgramError> {
    msg!("executing instruction {}", id);
    if id == 0 {
        permute_instruction_first(
            &mut main_account_data.state,
            &mut main_account_data.current_round,
            &mut main_account_data.current_round_index,
            &main_account_data.left,
            &main_account_data.right,
        )?;
    } else if id == 1 {
        permute_instruction_6(
            &mut main_account_data.state,
            &mut main_account_data.current_round,
            &mut main_account_data.current_round_index,
        )?;
    } else if id == 2 {
        permute_instruction_3(
            &mut main_account_data.state,
            &mut main_account_data.current_round,
            &mut main_account_data.current_round_index,
        )?;
    } else if id == 3 {
        permute_instruction_last(
            &mut main_account_data.state,
            &mut main_account_data.current_round,
            &mut main_account_data.current_round_index,
        )?;
    } else if id == 25 {
        insert_1_inner_loop(merkle_tree_account_data, main_account_data)?;
    } else if id == 14 {
        insert_0_double(merkle_tree_account_data, main_account_data)?;
    } else if id == 16 {
        insert_last_double(merkle_tree_account_data, main_account_data)?;
    }
    Ok(())
}

fn merkle_tree_pubkey_check(account_pubkey: Pubkey) -> Result<(), ProgramError> {
    if account_pubkey != solana_program::pubkey::Pubkey::new(&state::MERKLE_TREE_ACC_BYTES[..]) {
        msg!("invalid merkle tree");
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

fn pubkey_check(
    account_pubkey0: Pubkey,
    account_pubkey1: Pubkey,
    msg: String,
) -> Result<(), ProgramError> {
    if account_pubkey0 != account_pubkey1 {
        msg!(&msg);
        return Err(ProgramError::InvalidInstructionData);
    }

    Ok(())
}

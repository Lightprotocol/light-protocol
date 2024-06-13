#![cfg(not(target_os = "solana"))]
use std::collections::HashMap;

use crate::{
    delegate::{
        deposit::{
            DelegateAccountWithContext, DelegateAccountWithPackedContext,
            InputDelegateAccountWithPackedContext,
        },
        get_escrow_token_authority,
    },
    epoch::{
        claim_forester::CompressedForesterEpochAccountInput,
        sync_delegate::SyncDelegateTokenAccount,
    },
    protocol_config::state::ProtocolConfig,
    utils::{
        get_cpi_authority_pda, get_epoch_pda_address, get_forester_epoch_pda_address,
        get_forester_pda_address, get_forester_token_pool_pda, get_protocol_config_pda_address,
    },
    ForesterConfig, MINT,
};
use account_compression::{self, ID};
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use light_compressed_token::{
    get_token_pool_pda,
    process_transfer::{
        transfer_sdk::{
            create_input_output_and_remaining_accounts, create_input_token_accounts,
            to_account_metas,
        },
        InputTokenDataWithContext,
    },
    TokenData,
};
use light_macros::pubkey;
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{
        compressed_account::{
            pack_merkle_context, CompressedAccountWithMerkleContext, MerkleContext,
        },
        CompressedCpiContext,
    },
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
// TODO: move to non program sdk
pub const NOOP_PROGRAM_ID: Pubkey = pubkey!("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");

pub fn create_initialize_group_authority_instruction(
    signer_pubkey: Pubkey,
    group_accounts: Pubkey,
    seed_pubkey: Pubkey,
    authority: Pubkey,
) -> Instruction {
    let instruction_data = account_compression::instruction::InitializeGroupAuthority { authority };

    Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(signer_pubkey, true),
            AccountMeta::new(seed_pubkey, true),
            AccountMeta::new(group_accounts, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: instruction_data.data(),
    }
}

pub fn create_update_authority_instruction(
    signer_pubkey: Pubkey,
    new_authority: Pubkey,
    new_protocol_config: ProtocolConfig,
) -> Instruction {
    let authority_pda = get_protocol_config_pda_address();
    let update_authority_ix = crate::instruction::UpdateGovernanceAuthority {
        _bump: authority_pda.1,
        new_authority,
        new_config: new_protocol_config,
    };

    // update with new authority
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer_pubkey, true),
            AccountMeta::new(authority_pda.0, false),
        ],
        data: update_authority_ix.data(),
    }
}

pub fn create_register_program_instruction(
    signer_pubkey: Pubkey,
    authority_pda: (Pubkey, u8),
    group_account: Pubkey,
    program_id_to_be_registered: Pubkey,
) -> (Instruction, Pubkey) {
    let cpi_authority_pda = get_cpi_authority_pda();
    let registered_program_pda =
        Pubkey::find_program_address(&[program_id_to_be_registered.to_bytes().as_slice()], &ID).0;

    let register_program_ix = crate::instruction::RegisterSystemProgram {
        bump: cpi_authority_pda.1,
    };
    let register_program_accounts = crate::accounts::RegisteredProgram {
        authority: signer_pubkey,
        program_to_be_registered: program_id_to_be_registered,
        registered_program_pda,
        authority_pda: authority_pda.0,
        group_pda: group_account,
        cpi_authority: cpi_authority_pda.0,
        account_compression_program: ID,
        system_program: system_program::ID,
    };

    let instruction = Instruction {
        program_id: crate::ID,
        accounts: register_program_accounts.to_account_metas(Some(true)),
        data: register_program_ix.data(),
    };
    (instruction, registered_program_pda)
}

pub fn create_initialize_governance_authority_instruction(
    signer_pubkey: Pubkey,
    protocol_config: ProtocolConfig,
) -> Instruction {
    let authority_pda = get_protocol_config_pda_address();
    let ix = crate::instruction::InitializeGovernanceAuthority {
        bump: authority_pda.1,
        protocol_config,
    };
    let cpi_authority_pda = get_cpi_authority_pda().0;

    let accounts = crate::accounts::InitializeAuthority {
        authority_pda: authority_pda.0,
        authority: signer_pubkey,
        system_program: system_program::ID,
        mint: protocol_config.mint,
        cpi_authority: cpi_authority_pda,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: ix.data(),
    }
}

pub fn create_register_forester_instruction(
    governance_authority: &Pubkey,
    forester_authority: &Pubkey,
    config: ForesterConfig,
) -> Instruction {
    let (forester_pda, _bump) = get_forester_pda_address(forester_authority);
    let instruction_data = crate::instruction::RegisterForester { _bump: 0, config };
    let (protocol_config_pda, _) = get_protocol_config_pda_address();
    let token_pool_pda = get_forester_token_pool_pda(forester_authority);
    let accounts = crate::accounts::RegisterForester {
        forester_pda,
        signer: *governance_authority,
        protocol_config_pda,
        system_program: solana_sdk::system_program::id(),
        authority: *forester_authority,
        token_pool_pda,
        mint: MINT,
        cpi_authority_pda: get_cpi_authority_pda().0,
        token_program: anchor_spl::token::ID,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_update_forester_epoch_pda_instruction(
    forester_authority: &Pubkey,
    new_authority: &Pubkey,
) -> Instruction {
    let (forester_epoch_pda, _bump) = get_forester_pda_address(forester_authority);
    let instruction_data = crate::instruction::UpdateForesterEpochPda {
        authority: *new_authority,
    };
    let accounts = crate::accounts::UpdateForesterEpochPda {
        forester_epoch_pda,
        signer: *forester_authority,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_update_forester_pda_instruction(
    forester_authority: &Pubkey,
    new_authority: Option<Pubkey>,
    config: ForesterConfig,
) -> Instruction {
    let (forester_pda, _) = get_forester_pda_address(forester_authority);
    let instruction_data = crate::instruction::UpdateForester { config };
    let accounts = crate::accounts::UpdateForester {
        forester_pda,
        authority: *forester_authority,
        new_authority,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_register_forester_epoch_pda_instruction(
    authority: &Pubkey,
    epoch: u64,
) -> Instruction {
    let (forester_pda, _) = get_forester_pda_address(authority);
    let (forester_epoch_pda, _bump) = get_forester_epoch_pda_address(&forester_pda, epoch);

    let epoch_pda = get_epoch_pda_address(epoch);
    let protocol_config_pda = get_protocol_config_pda_address().0;
    let instruction_data = crate::instruction::RegisterForesterEpoch { epoch };
    let accounts = crate::accounts::RegisterForesterEpoch {
        forester_epoch_pda,
        forester_pda,
        authority: *authority,
        epoch_pda,
        protocol_config: protocol_config_pda,
        system_program: solana_sdk::system_program::id(),
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_finalize_registration_instruction(authority: &Pubkey, epoch: u64) -> Instruction {
    let forester_pda = get_forester_pda_address(authority).0;
    let (forester_epoch_pda, _bump) = get_forester_epoch_pda_address(&forester_pda, epoch);
    let epoch_pda = get_epoch_pda_address(epoch);
    let instruction_data = crate::instruction::FinalizeRegistration {};
    let accounts = crate::accounts::FinalizeRegistration {
        forester_epoch_pda,
        authority: *authority,
        epoch_pda,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_report_work_instruction(authority: &Pubkey, epoch: u64) -> Instruction {
    let (forester_pda, _) = get_forester_pda_address(authority);
    let (forester_epoch_pda, _bump) = get_forester_epoch_pda_address(&forester_pda, epoch);
    let epoch_pda = get_epoch_pda_address(epoch);
    let instruction_data = crate::instruction::ReportWork {};
    let accounts = crate::accounts::ReportWork {
        authority: *authority,
        forester_epoch_pda,
        epoch_pda,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub struct StandardCompressedTokenProgramAccounts {
    pub token_cpi_authority_pda: Pubkey,
    pub compressed_token_program: Pubkey,
    pub token_pool_pda: Pubkey,
    pub token_program: Pubkey,
    pub light_system_program: Pubkey,
    pub registered_program_pda: Pubkey,
    pub noop_program: Pubkey,
    pub account_compression_authority: Pubkey,
    pub account_compression_program: Pubkey,
    pub system_program: Pubkey,
}

pub fn get_standard_compressed_token_program_accounts(
    mint: Pubkey,
) -> StandardCompressedTokenProgramAccounts {
    StandardCompressedTokenProgramAccounts {
        token_cpi_authority_pda: light_compressed_token::process_transfer::get_cpi_authority_pda()
            .0,
        compressed_token_program: light_compressed_token::ID,
        token_pool_pda: light_compressed_token::get_token_pool_pda(&mint),
        token_program: anchor_spl::token::ID,
        light_system_program: light_system_program::ID,
        registered_program_pda: light_system_program::utils::get_registered_program_pda(
            &light_system_program::ID,
        ),
        noop_program: NOOP_PROGRAM_ID,
        account_compression_authority: light_system_program::utils::get_cpi_authority_pda(
            &light_system_program::ID,
        ),
        account_compression_program: account_compression::ID,
        system_program: system_program::ID,
    }
}

pub fn create_mint_to_instruction(
    mint: &Pubkey,
    authority: &Pubkey,
    recipient: &Pubkey,
    amount: u64,
    merkle_tree: &Pubkey,
) -> Instruction {
    let protocol_config_pda = get_protocol_config_pda_address().0;
    let (cpi_authority_pda, _) = get_cpi_authority_pda();
    let instruction_data = crate::instruction::Mint {
        amounts: vec![amount],
        recipients: vec![*recipient],
    };
    let standard_accounts = get_standard_compressed_token_program_accounts(*mint);
    let accounts = crate::accounts::Mint {
        fee_payer: *authority,
        authority: *authority,
        protocol_config_pda,
        mint: *mint,
        merkle_tree: *merkle_tree,
        cpi_authority: cpi_authority_pda,
        token_cpi_authority_pda: standard_accounts.token_cpi_authority_pda,
        compressed_token_program: standard_accounts.compressed_token_program,
        token_pool_pda: standard_accounts.token_pool_pda,
        token_program: standard_accounts.token_program,
        light_system_program: standard_accounts.light_system_program,
        registered_program_pda: standard_accounts.registered_program_pda,
        noop_program: standard_accounts.noop_program,
        account_compression_authority: standard_accounts.account_compression_authority,
        account_compression_program: standard_accounts.account_compression_program,
        system_program: standard_accounts.system_program,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub struct StandardRegistryAccounts {
    pub protocol_config_pda: Pubkey,
    pub cpi_authority_pda: Pubkey,
    pub self_program: Pubkey,
}

pub fn get_standard_registry_accounts() -> StandardRegistryAccounts {
    StandardRegistryAccounts {
        protocol_config_pda: get_protocol_config_pda_address().0,
        cpi_authority_pda: get_cpi_authority_pda().0,
        self_program: crate::ID,
    }
}

#[derive(Debug, Clone)]
pub struct CreateDepositInstructionInputs {
    pub sender: Pubkey,
    pub cpi_context_account: Pubkey,
    pub salt: u64,
    pub delegate_account: Option<DelegateAccountWithContext>,
    pub amount: u64,
    pub input_token_data: Vec<TokenData>,
    pub input_compressed_accounts: Vec<CompressedAccountWithMerkleContext>,
    pub input_escrow_token_account: Option<(TokenData, CompressedAccountWithMerkleContext)>,
    pub escrow_token_account_merkle_tree: Pubkey,
    pub change_compressed_account_merkle_tree: Pubkey,
    pub output_delegate_compressed_account_merkle_tree: Pubkey,
    pub proof: CompressedProof,
    pub root_indices: Vec<u16>,
}

pub fn get_index_and_add_to_remaining_accounts(
    remaining_accounts: &mut HashMap<Pubkey, usize>,
    account: &Pubkey,
) -> usize {
    let index = remaining_accounts.len();

    match remaining_accounts.get(account) {
        Some(index) => *index,
        None => {
            remaining_accounts.insert(*account, index);
            index
        }
    }
}

/// Accounts in proof order:
/// 1. input pda (if some)
/// 2. input token accounts[..]
/// 3. input escrow token account (if some)
///
/// 3 types of input accounts
/// 1. input_token_data
/// 2.
// input_escrow_token_account is expected to be the last account in the proof
pub fn create_deposit_instruction<const IS_DEPOSIT: bool>(
    inputs: CreateDepositInstructionInputs,
) -> Instruction {
    let root_indices_range_input_accounts = 0..inputs.input_compressed_accounts.len();

    let (mut remaining_accounts, input_compressed_token_accounts, _) =
        create_input_output_and_remaining_accounts(
            &[],
            &inputs.input_token_data,
            inputs.input_compressed_accounts.as_slice(),
            &inputs.root_indices[root_indices_range_input_accounts],
            &[], // outputs are created onchain
        );
    let input_escrow_token_account =
        if let Some((token_data, compressed_account)) = inputs.input_escrow_token_account {
            let mut index = remaining_accounts.len();
            let mut input_token_data_with_context: Vec<InputTokenDataWithContext> = Vec::new();
            create_input_token_accounts(
                &[token_data],
                &mut remaining_accounts,
                &[compressed_account],
                &mut index,
                &inputs.root_indices[inputs.input_compressed_accounts.len()
                    ..inputs.input_compressed_accounts.len() + 1],
                &mut input_token_data_with_context,
            );
            Some(input_token_data_with_context[0].clone())
        } else {
            None
        };
    let escrow_token_account_merkle_tree_index = get_index_and_add_to_remaining_accounts(
        &mut remaining_accounts,
        &inputs.escrow_token_account_merkle_tree,
    ) as u8;
    let change_compressed_account_merkle_tree_index = get_index_and_add_to_remaining_accounts(
        &mut remaining_accounts,
        &inputs.change_compressed_account_merkle_tree,
    ) as u8;
    let output_delegate_compressed_account_merkle_tree_index =
        get_index_and_add_to_remaining_accounts(
            &mut remaining_accounts,
            &inputs.output_delegate_compressed_account_merkle_tree,
        ) as u8;
    let cpi_context_account_index = get_index_and_add_to_remaining_accounts(
        &mut remaining_accounts,
        &inputs.cpi_context_account,
    ) as u8;
    let delegate_account = if let Some(delegate_account) = inputs.delegate_account {
        let packed_merkle_context =
            pack_merkle_context(&[delegate_account.merkle_context], &mut remaining_accounts);

        Some(InputDelegateAccountWithPackedContext {
            delegate_account: delegate_account.delegate_account.into(),
            merkle_context: packed_merkle_context[0],
            root_index: inputs.root_indices[inputs.root_indices.len() - 1],
        })
    } else {
        None
    };
    let instruction_data = if IS_DEPOSIT {
        crate::instruction::Deposit {
            salt: inputs.salt,
            delegate_account,
            deposit_amount: inputs.amount,
            input_compressed_token_accounts,
            input_escrow_token_account,
            escrow_token_account_merkle_tree_index,
            change_compressed_account_merkle_tree_index,
            output_delegate_compressed_account_merkle_tree_index,
            proof: inputs.proof,
            cpi_context: CompressedCpiContext {
                set_context: false,
                first_set_context: true,
                cpi_context_account_index,
            },
        }
        .data()
    } else {
        let delegate_account = if let Some(delegate_account) = delegate_account {
            delegate_account
        } else {
            panic!("delegate account is required for withdrawal");
        };
        let input_escrow_token_account =
            if let Some(input_escrow_token_account) = input_escrow_token_account {
                input_escrow_token_account
            } else {
                panic!("input escrow token account is required for withdrawal");
            };
        crate::instruction::Withdrawal {
            salt: inputs.salt,
            delegate_account,
            withdrawal_amount: inputs.amount,
            input_escrow_token_account,
            escrow_token_account_merkle_tree_index,
            change_compressed_account_merkle_tree_index,
            output_delegate_compressed_account_merkle_tree_index,
            proof: inputs.proof,
            cpi_context: CompressedCpiContext {
                set_context: false,
                first_set_context: true,
                cpi_context_account_index,
            },
        }
        .data()
    };
    let standard_accounts = get_standard_compressed_token_program_accounts(MINT);
    let (cpi_authority_pda, _) = get_cpi_authority_pda();
    let standard_registry_accounts = get_standard_registry_accounts();

    let escrow_token_authority = get_escrow_token_authority(&inputs.sender, inputs.salt).0;
    let accounts = crate::accounts::DepositOrWithdrawInstruction {
        fee_payer: inputs.sender,
        authority: inputs.sender,
        cpi_authority: cpi_authority_pda,
        token_cpi_authority_pda: standard_accounts.token_cpi_authority_pda,
        compressed_token_program: standard_accounts.compressed_token_program,
        light_system_program: standard_accounts.light_system_program,
        registered_program_pda: standard_accounts.registered_program_pda,
        noop_program: standard_accounts.noop_program,
        account_compression_authority: standard_accounts.account_compression_authority,
        account_compression_program: standard_accounts.account_compression_program,
        system_program: standard_accounts.system_program,
        cpi_context_account: inputs.cpi_context_account,
        invoking_program: standard_registry_accounts.self_program,
        escrow_token_authority,
        protocol_config: standard_registry_accounts.protocol_config_pda,
        self_program: standard_registry_accounts.self_program,
    };
    let remaining_accounts = to_account_metas(remaining_accounts);
    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data,
    }
}

#[derive(Debug, Clone)]
pub struct CreateDelegateInstructionInputs {
    pub sender: Pubkey,
    pub delegate_account: DelegateAccountWithContext,
    pub amount: u64,
    pub output_delegate_compressed_account_merkle_tree: Pubkey,
    pub proof: CompressedProof,
    pub root_index: u16,
    pub no_sync: bool,
    pub forester_pda: Pubkey,
}

pub fn create_delegate_instruction<const IS_DELEGATE: bool>(
    inputs: CreateDelegateInstructionInputs,
) -> Instruction {
    let mut remaining_accounts = HashMap::new();
    let output_merkle_tree_index = get_index_and_add_to_remaining_accounts(
        &mut remaining_accounts,
        &inputs.output_delegate_compressed_account_merkle_tree,
    ) as u8;

    let packed_merkle_context = pack_merkle_context(
        &[inputs.delegate_account.merkle_context],
        &mut remaining_accounts,
    );

    let delegate_account = DelegateAccountWithPackedContext {
        delegate_account: inputs.delegate_account.delegate_account,
        merkle_context: packed_merkle_context[0],
        root_index: inputs.root_index,
        output_merkle_tree_index,
    };
    let instruction_data = if IS_DELEGATE {
        crate::instruction::Delegate {
            delegate_account,
            delegate_amount: inputs.amount,
            proof: inputs.proof,
            no_sync: inputs.no_sync,
        }
        .data()
    } else {
        crate::instruction::Undelegate {
            delegate_account,
            delegate_amount: inputs.amount,
            proof: inputs.proof,
            no_sync: inputs.no_sync,
        }
        .data()
    };
    let standard_accounts = get_standard_compressed_token_program_accounts(MINT);
    let (cpi_authority_pda, _) = get_cpi_authority_pda();
    let standard_registry_accounts = get_standard_registry_accounts();

    let accounts = crate::accounts::DelegatetOrUndelegateInstruction {
        fee_payer: inputs.sender,
        authority: inputs.sender,
        cpi_authority: cpi_authority_pda,
        light_system_program: standard_accounts.light_system_program,
        registered_program_pda: standard_accounts.registered_program_pda,
        noop_program: standard_accounts.noop_program,
        account_compression_authority: standard_accounts.account_compression_authority,
        account_compression_program: standard_accounts.account_compression_program,
        system_program: standard_accounts.system_program,
        invoking_program: standard_registry_accounts.self_program,
        protocol_config: standard_registry_accounts.protocol_config_pda,
        self_program: standard_registry_accounts.self_program,
        forester_pda: inputs.forester_pda,
    };
    let remaining_accounts = to_account_metas(remaining_accounts);
    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data,
    }
}

pub fn create_forester_claim_instruction(
    forester_pubkey: Pubkey,
    epoch: u64,
    output_merkle_tree: Pubkey,
) -> Instruction {
    let instruction_data = crate::instruction::ClaimForesterRewards {};

    let standard_accounts = get_standard_compressed_token_program_accounts(MINT);
    let (cpi_authority_pda, _) = get_cpi_authority_pda();
    let standard_registry_accounts = get_standard_registry_accounts();

    let forester_pda = get_forester_pda_address(&forester_pubkey).0;
    let forester_epoch_pda = get_forester_epoch_pda_address(&forester_pda, epoch).0;
    let forester_token_pool = get_forester_token_pool_pda(&forester_pubkey);
    let epoch_pda = get_epoch_pda_address(epoch);
    let accounts = crate::accounts::ClaimForesterInstruction {
        fee_payer: forester_pubkey,
        authority: forester_pubkey,
        cpi_authority: cpi_authority_pda,
        token_cpi_authority_pda: standard_accounts.token_cpi_authority_pda,
        compressed_token_program: standard_accounts.compressed_token_program,
        light_system_program: standard_accounts.light_system_program,
        registered_program_pda: standard_accounts.registered_program_pda,
        noop_program: standard_accounts.noop_program,
        account_compression_authority: standard_accounts.account_compression_authority,
        account_compression_program: standard_accounts.account_compression_program,
        system_program: standard_accounts.system_program,
        invoking_program: standard_registry_accounts.self_program,
        self_program: standard_registry_accounts.self_program,
        forester_token_pool,
        forester_epoch_pda,
        forester_pda,
        spl_token_program: anchor_spl::token::ID,
        epoch_pda,
        mint: MINT,
        output_merkle_tree,
        compression_token_pool: get_token_pool_pda(&MINT),
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

#[derive(Debug, Clone)]
pub struct CreateSyncDelegateInstructionInputs {
    pub sender: Pubkey,
    pub cpi_context_account: Pubkey,
    pub salt: u64,
    pub delegate_account: DelegateAccountWithContext,
    // pub input_compressed_accounts: Vec<CompressedAccountWithMerkleContext>,
    pub input_escrow_token_account: Option<(TokenData, CompressedAccountWithMerkleContext)>,
    pub output_token_account_merkle_tree: Pubkey,
    // pub change_compressed_account_merkle_tree: Pubkey,
    pub output_delegate_compressed_account_merkle_tree: Pubkey,
    pub proof: CompressedProof,
    pub root_indices: Vec<u16>,
    pub forester_pubkey: Pubkey,
    pub previous_hash: [u8; 32],
    pub compressed_forester_epoch_pdas: Vec<CompressedForesterEpochAccountInput>,
    pub sync_delegate_token_account: bool,
    pub last_account_merkle_context: MerkleContext,
    pub last_account_root_index: u16,
}

pub fn create_sync_delegate_instruction(
    inputs: CreateSyncDelegateInstructionInputs,
) -> Instruction {
    let mut remaining_accounts = HashMap::new();

    let output_merkle_tree_index = get_index_and_add_to_remaining_accounts(
        &mut remaining_accounts,
        &inputs.output_delegate_compressed_account_merkle_tree,
    ) as u8;
    let delegate_account = {
        let delegate_account = inputs.delegate_account;
        let packed_merkle_context =
            pack_merkle_context(&[delegate_account.merkle_context], &mut remaining_accounts);
        DelegateAccountWithPackedContext {
            delegate_account: delegate_account.delegate_account.into(),
            merkle_context: packed_merkle_context[0],
            root_index: inputs.root_indices[0],
            output_merkle_tree_index,
        }
    };
    let last_account_merkle_context = pack_merkle_context(
        &[inputs.last_account_merkle_context],
        &mut remaining_accounts,
    )[0];

    let sync_delegate_token_account = SyncDelegateTokenAccount {
        salt: inputs.salt,
        cpi_context: CompressedCpiContext {
            set_context: false,
            first_set_context: true,
            cpi_context_account_index: get_index_and_add_to_remaining_accounts(
                &mut remaining_accounts,
                &inputs.cpi_context_account,
            ) as u8,
        },
    };

    let (
        escrow_token_authority,
        cpi_context_account,
        compressed_token_program,
        forester_token_pool,
        token_cpi_authority_pda,
        sync_delegate_token_account,
        input_escrow_token_account,
        spl_token_pool,
        spl_token_program,
        output_token_account_merkle_tree_index,
    ) = if let Some((token_data, compressed_account)) = inputs.input_escrow_token_account {
        let mut index = remaining_accounts.len();
        let mut input_token_data_with_context: Vec<InputTokenDataWithContext> = Vec::new();
        create_input_token_accounts(
            &[token_data],
            &mut remaining_accounts,
            &[compressed_account],
            &mut index,
            &[inputs.root_indices[1]],
            &mut input_token_data_with_context,
        );
        let output_token_account_merkle_tree_index = get_index_and_add_to_remaining_accounts(
            &mut remaining_accounts,
            &inputs.output_token_account_merkle_tree,
        ) as u8;
        let standard_accounts = get_standard_compressed_token_program_accounts(MINT);
        (
            Some(get_escrow_token_authority(&inputs.sender, inputs.salt).0),
            Some(inputs.cpi_context_account),
            Some(standard_accounts.compressed_token_program),
            Some(get_forester_token_pool_pda(&inputs.forester_pubkey)),
            Some(standard_accounts.token_cpi_authority_pda),
            Some(sync_delegate_token_account),
            Some(input_token_data_with_context[0].clone()),
            Some(get_token_pool_pda(&MINT)),
            Some(anchor_spl::token::ID),
            output_token_account_merkle_tree_index,
        )
    } else {
        (None, None, None, None, None, None, None, None, None, 0)
    };
    let forester_pda_pubkey = get_forester_pda_address(&inputs.forester_pubkey).0;
    let instruction_data = crate::instruction::SyncDelegate {
        _salt: inputs.salt,
        input_escrow_token_account,
        delegate_account,
        forester_pda_pubkey,
        previous_hash: inputs.previous_hash,
        compressed_forester_epoch_pdas: inputs.compressed_forester_epoch_pdas,
        last_account_merkle_context,
        last_account_root_index: inputs.last_account_root_index,
        output_token_account_merkle_tree_index,
        inclusion_proof: inputs.proof,
        sync_delegate_token_account,
    };

    let standard_accounts = get_standard_compressed_token_program_accounts(MINT);
    let (cpi_authority_pda, _) = get_cpi_authority_pda();
    let standard_registry_accounts = get_standard_registry_accounts();

    let accounts = crate::accounts::SyncDelegateInstruction {
        fee_payer: inputs.sender,
        authority: inputs.sender,
        cpi_authority: cpi_authority_pda,
        light_system_program: standard_accounts.light_system_program,
        registered_program_pda: standard_accounts.registered_program_pda,
        noop_program: standard_accounts.noop_program,
        account_compression_authority: standard_accounts.account_compression_authority,
        account_compression_program: standard_accounts.account_compression_program,
        system_program: standard_accounts.system_program,
        self_program: standard_registry_accounts.self_program,
        protocol_config: standard_registry_accounts.protocol_config_pda,
        escrow_token_authority,
        cpi_context_account,
        token_cpi_authority_pda,
        compressed_token_program,
        forester_token_pool,
        spl_token_pool,
        spl_token_program,
    };
    let remaining_accounts = to_account_metas(remaining_accounts);
    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    }
}

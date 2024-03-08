use anchor_lang::{prelude::*, AnchorDeserialize};
use light_hasher::{errors::HasherError, DataHasher, Hasher, Poseidon};
use light_utils::hash_to_bn254_field_size_le;
use psp_compressed_pda::{
    tlv::{Tlv, TlvDataElement},
    utils::CompressedProof,
    utxo::{InUtxoTuple, OutUtxo, OutUtxoTuple},
    InstructionDataTransfer as PspCompressedPdaInstructionDataTransfer,
};

use crate::ErrorCode;

/// Process a token transfer instruction
///
/// 1. check signer / delegate
/// 2. if is delegate check delegated amount and decrease it, there needs to be an out utxo with the same utxo data as the in utxo
/// 3. check in utxos are of same mint
/// 4. check sum of in utxo is equal to sum of out utxos
/// 5.1 create_out_utxos
/// 5.2 create delegate change utxos
/// 6. serialize and add tlv data to in utxos
/// 7. invoke psp_compressed_pda::execute_compressed_transaction
pub fn process_transfer<'a, 'b, 'c, 'info: 'b + 'c>(
    ctx: Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    inputs: Vec<u8>,
) -> Result<()> {
    let mut inputs: InstructionDataTransfer =
        InstructionDataTransfer::deserialize(&mut inputs.as_slice())?;

    let is_delegate = check_signer_or_delegate(&ctx.accounts.authority.key(), &inputs.in_tlv_data)?;
    if is_delegate {
        unimplemented!("delegate check not implemented");
    }

    let mint = check_mint(&inputs.in_tlv_data)?;

    sum_check(
        &inputs.in_tlv_data,
        &inputs
            .out_utxos
            .iter()
            .map(|utxo| utxo.amount)
            .collect::<Vec<u64>>(),
        None,
        true,
    )?;

    let out_utxos = crate::create_out_utxos(
        mint,
        inputs
            .out_utxos
            .iter()
            .map(|utxo| utxo.owner)
            .collect::<Vec<Pubkey>>()
            .as_slice(),
        inputs
            .out_utxos
            .iter()
            .map(|utxo: &TokenTransferOutUtxo| utxo.amount)
            .collect::<Vec<u64>>()
            .as_slice(),
    );
    // TODO: add create delegate change utxos
    add_tlv_to_in_utxos(&mut inputs.in_utxos, inputs.in_tlv_data.as_slice())?;

    cpi_execute_compressed_transaction_transfer(
        &ctx,
        inputs.in_utxos,
        inputs.root_indices,
        &out_utxos,
    )?;
    Ok(())
}

pub fn add_tlv_to_in_utxos(
    in_utxos: &mut [InUtxoTuple],
    in_tlv_data: &[TokenTlvData],
) -> Result<()> {
    for (i, in_utxo) in in_utxos.iter_mut().enumerate() {
        let tlv_data = TlvDataElement {
            discriminator: 2u64.to_le_bytes(),
            owner: in_utxo.in_utxo.owner,
            data: in_tlv_data[i].try_to_vec().unwrap(),
            data_hash: in_tlv_data[i].hash().unwrap(),
        };
        let tlv = Tlv {
            tlv_elements: vec![tlv_data],
        };
        in_utxo.in_utxo.data = Some(tlv);
    }
    Ok(())
}

#[inline(never)]
pub fn cpi_execute_compressed_transaction_transfer<'info>(
    ctx: &Context<'_, '_, '_, 'info, TransferInstruction<'info>>,
    in_utxos: Vec<InUtxoTuple>,
    root_indices: Vec<u16>,
    out_utxos: &[OutUtxo],
) -> Result<()> {
    let mut _out_utxos = Vec::<OutUtxoTuple>::new();
    for utxo in out_utxos.iter() {
        _out_utxos.push(OutUtxoTuple {
            out_utxo: utxo.clone(),
            index_mt_account: 0,
        });
    }

    let inputs_struct = PspCompressedPdaInstructionDataTransfer {
        low_element_indices: Vec::new(),
        relay_fee: None,
        in_utxos,
        out_utxos: _out_utxos,
        root_indices,
        proof: None,
    };

    let mut inputs = Vec::new();
    PspCompressedPdaInstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    let (_, bump) = get_cpi_authority_pda();
    let bump = &[bump];
    let seeds = [b"cpi_authority".as_slice(), bump];

    let signer_seeds = &[&seeds[..]];
    let cpi_accounts = psp_compressed_pda::cpi::accounts::TransferInstruction {
        signer: ctx.accounts.cpi_authority_pda.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        psp_account_compression_authority: ctx
            .accounts
            .psp_account_compression_authority
            .to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        cpi_signature_account: None,
        invoking_program: Some(ctx.accounts.self_program.to_account_info()),
    };
    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.compressed_pda_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    psp_compressed_pda::cpi::execute_compressed_transaction(cpi_ctx, inputs)?;
    Ok(())
}

fn check_signer_or_delegate(signer: &Pubkey, tlv: &[TokenTlvData]) -> Result<bool> {
    let mut is_delegate = false;
    for utxo in tlv {
        if utxo.owner == *signer {
        } else if utxo.delegate.is_some() && utxo.delegate.unwrap() == *signer {
            is_delegate = true;
        } else {
            msg!(
                "Signer check failed utxo.owner {:?} != authority {:?}",
                utxo.owner,
                signer
            );
            return Err(ErrorCode::SignerCheckFailed.into());
        }
    }
    Ok(is_delegate)
}

fn check_mint(tlv: &[TokenTlvData]) -> Result<Pubkey> {
    let mint = tlv[0].mint;
    for utxo in tlv {
        if utxo.mint != mint {
            return Err(ErrorCode::MintCheckFailed.into());
        }
    }
    Ok(mint)
}

pub fn sum_check(
    in_tlvs: &[TokenTlvData],
    out_amounts: &[u64],
    compression_amount: Option<&u64>,
    is_compress: bool,
) -> anchor_lang::Result<()> {
    let mut sum: u64 = 0;
    for in_tlv in in_tlvs.iter() {
        sum = sum
            .checked_add(in_tlv.amount)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| ErrorCode::ComputeInputSumFailed)?;
    }

    for amount in out_amounts.iter() {
        sum = sum
            .checked_sub(*amount)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| ErrorCode::ComputeOutputSumFailed)?;
    }

    if let Some(compression_amount) = compression_amount {
        if is_compress {
            sum = sum
                .checked_add(*compression_amount)
                .ok_or(ProgramError::ArithmeticOverflow)
                .map_err(|_| ErrorCode::ComputeCompressSumFailed)?;
        } else {
            sum = sum
                .checked_sub(*compression_amount)
                .ok_or(ProgramError::ArithmeticOverflow)
                .map_err(|_| ErrorCode::ComputeDecompressSumFailed)?;
        }
    }

    if sum == 0 {
        Ok(())
    } else {
        Err(ErrorCode::SumCheckFailed.into())
    }
}

#[derive(Accounts)]
pub struct TransferInstruction<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
    // This is the cpi signer
    /// CHECK: that mint authority is derived from signer
    #[account(seeds = [b"cpi_authority"], bump,)]
    pub cpi_authority_pda: UncheckedAccount<'info>,
    pub compressed_pda_program: Program<'info, psp_compressed_pda::program::PspCompressedPda>,
    /// CHECK: this account
    #[account(mut)]
    pub registered_program_pda: UncheckedAccount<'info>,
    /// CHECK: this account
    pub noop_program: UncheckedAccount<'info>,
    /// CHECK: this account in psp account compression program
    #[account(mut, seeds = [b"cpi_authority", account_compression::ID.to_bytes().as_slice()], bump, seeds::program = psp_compressed_pda::ID,)]
    pub psp_account_compression_authority: UncheckedAccount<'info>,
    /// CHECK: this account in psp account compression program
    pub account_compression_program:
        Program<'info, account_compression::program::AccountCompression>,
    pub self_program: Program<'info, crate::program::PspCompressedToken>,
}

// TODO: remove when instruction data struct has been implemented with beet in client
// ported from utxo.rs so that it is included in anchor idl
#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct UtxoClient {
    pub owner: Pubkey,
    pub blinding: [u8; 32],
    pub lamports: u64,
    pub data: Option<Tlv>,
}

// TODO: remove when instruction data struct has been implemented with beet in client
// ported from utxo.rs so that it is included in anchor idl
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InUtxoTupleClient {
    pub in_utxo: UtxoClient,
    pub index_mt_account: u8,
    pub index_nullifier_array_account: u8,
}

// TODO: remove when instruction data struct has been implemented with beet in client
// ported from utxo.rs so that it is included in anchor idl
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedProofClient {
    pub a: [u8; 32],
    pub b: [u8; 64],
    pub c: [u8; 32],
}

// TODO: remove when instruction data struct has been implemented with beet in client
// this struct is just used in the client
// for that reason it uses the ported client structs
#[derive(Debug)]
#[account]
pub struct InstructionDataTransferClient {
    proof: Option<CompressedProofClient>,
    root_indices: Vec<u16>,
    in_utxos: Vec<InUtxoTupleClient>,
    in_tlv_data: Vec<TokenTlvDataClient>,
    out_utxos: Vec<TokenTransferOutUtxo>,
}

// TODO: parse utxos a more efficient way, since owner is sent multiple times this way
// This struct is equivalent to the InstructionDataTransferClient, but uses the imported types from the psp_compressed_pda
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InstructionDataTransfer {
    proof: Option<CompressedProof>,
    root_indices: Vec<u16>,
    in_utxos: Vec<InUtxoTuple>,
    in_tlv_data: Vec<TokenTlvData>,
    out_utxos: Vec<TokenTransferOutUtxo>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct TokenTransferOutUtxo {
    pub owner: Pubkey,
    pub amount: u64,
    pub lamports: Option<u64>,
    pub index_mt_account: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum AccountState {
    Uninitialized,
    Initialized,
    Frozen,
}

#[derive(Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct TokenTlvData {
    /// The mint associated with this account
    pub mint: Pubkey,
    /// The owner of this account.
    pub owner: Pubkey,
    /// The amount of tokens this account holds.
    pub amount: u64,
    /// If `delegate` is `Some` then `delegated_amount` represents
    /// the amount authorized by the delegate
    pub delegate: Option<Pubkey>,
    /// The account's state
    pub state: AccountState,
    /// If is_some, this is a native token, and the value logs the rent-exempt
    /// reserve. An Account is required to be rent-exempt, so the value is
    /// used by the Processor to ensure that wrapped SOL accounts do not
    /// drop below this threshold.
    pub is_native: Option<u64>,
    /// The amount delegated
    pub delegated_amount: u64,
    // TODO: validate that we don't need close authority
    // /// Optional authority to close the account.
    // pub close_authority: Option<Pubkey>,
}
#[derive(Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct TokenTlvDataClient {
    /// The mint associated with this account
    pub mint: Pubkey,
    /// The owner of this account.
    pub owner: Pubkey,
    /// The amount of tokens this account holds.
    pub amount: u64,
    /// If `delegate` is `Some` then `delegated_amount` represents
    /// the amount authorized by the delegate
    pub delegate: Option<Pubkey>,
    /// The account's state
    pub state: u8,
    /// If is_some, this is a native token, and the value logs the rent-exempt
    /// reserve. An Account is required to be rent-exempt, so the value is
    /// used by the Processor to ensure that wrapped SOL accounts do not
    /// drop below this threshold.
    pub is_native: Option<u64>,
    /// The amount delegated
    pub delegated_amount: u64,
    // TODO: validate that we don't need close authority
    // /// Optional authority to close the account.
    // pub close_authority: Option<Pubkey>,
}

impl DataHasher for TokenTlvData {
    fn hash(&self) -> std::result::Result<[u8; 32], HasherError> {
        let delegate = match self.delegate {
            Some(delegate) => {
                hash_to_bn254_field_size_le(delegate.to_bytes().as_slice())
                    .unwrap()
                    .0
            }
            None => [0u8; 32],
        };
        // let close_authority = match self.close_authority {
        //     Some(close_authority) => {
        //         hash_to_bn254_field_size_le(close_authority.to_bytes().as_slice())
        //             .unwrap()
        //             .0
        //     }
        //     None => [0u8; 32],
        // };

        Poseidon::hashv(&[
            &hash_to_bn254_field_size_le(self.mint.to_bytes().as_slice())
                .unwrap()
                .0,
            &hash_to_bn254_field_size_le(self.owner.to_bytes().as_slice())
                .unwrap()
                .0,
            &self.amount.to_le_bytes(),
            &delegate,
            &(self.state as u8).to_le_bytes(),
            &self.is_native.unwrap_or_default().to_le_bytes(),
            &self.delegated_amount.to_le_bytes(),
            // &close_authority,
        ])
    }
}

pub fn get_cpi_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"cpi_authority"], &crate::ID)
}

#[cfg(not(target_os = "solana"))]
pub mod transfer_sdk {
    use std::collections::HashMap;

    use account_compression::{AccountMeta, NOOP_PROGRAM_ID};
    use anchor_lang::{AnchorDeserialize, AnchorSerialize, InstructionData, ToAccountMetas};
    use psp_compressed_pda::{
        utils::CompressedProof,
        utxo::{InUtxoTuple, Utxo},
    };
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

    use crate::{InstructionDataTransfer, TokenTransferOutUtxo};
    #[allow(clippy::too_many_arguments)]
    pub fn create_transfer_instruction(
        fee_payer: &Pubkey,
        authority: &Pubkey,
        in_utxo_merkle_tree_pubkeys: &[Pubkey],
        nullifier_array_pubkeys: &[Pubkey],
        out_utxo_merkle_tree_pubkeys: &[Pubkey],
        in_utxos: &[Utxo],
        out_utxos: &[TokenTransferOutUtxo],
        root_indices: &[u16],
        proof: &CompressedProof,
    ) -> Instruction {
        let mut out_utxos = out_utxos.to_vec();
        let mut remaining_accounts = HashMap::<Pubkey, usize>::new();
        let mut _in_utxos: Vec<InUtxoTuple> = Vec::<InUtxoTuple>::new();
        let mut in_utxo_tlv_data: Vec<crate::TokenTlvData> = Vec::new();
        for (i, mt) in in_utxo_merkle_tree_pubkeys.iter().enumerate() {
            match remaining_accounts.get(mt) {
                Some(_) => {}
                None => {
                    remaining_accounts.insert(*mt, i);
                }
            };
            let mut in_utxo = in_utxos[i].clone();
            let token_tlv_data = crate::TokenTlvData::deserialize(
                &mut in_utxo.data.unwrap().tlv_elements[0].data.as_slice(),
            )
            .unwrap();
            in_utxo_tlv_data.push(token_tlv_data);
            in_utxo.data = None;
            _in_utxos.push(InUtxoTuple {
                in_utxo,
                index_mt_account: *remaining_accounts.get(mt).unwrap() as u8,
                index_nullifier_array_account: 0,
            });
        }
        let len: usize = remaining_accounts.len();
        for (i, mt) in nullifier_array_pubkeys.iter().enumerate() {
            match remaining_accounts.get(mt) {
                Some(_) => {}
                None => {
                    remaining_accounts.insert(*mt, i + len);
                }
            };
            _in_utxos[i].index_nullifier_array_account = *remaining_accounts.get(mt).unwrap() as u8;
        }
        let len: usize = remaining_accounts.len();

        for (i, mt) in out_utxo_merkle_tree_pubkeys.iter().enumerate() {
            match remaining_accounts.get(mt) {
                Some(_) => {}
                None => {
                    remaining_accounts.insert(*mt, i + len);
                }
            };
            out_utxos[i].index_mt_account = *remaining_accounts.get(mt).unwrap() as u8;
        }

        let mut remaining_accounts = remaining_accounts
            .iter()
            .map(|(k, i)| (AccountMeta::new(*k, false), *i))
            .collect::<Vec<(AccountMeta, usize)>>();
        // hash maps are not sorted so we need to sort manually and collect into a vector again
        remaining_accounts.sort_by(|a, b| a.1.cmp(&b.1));
        let remaining_accounts = remaining_accounts
            .iter()
            .map(|(k, _)| k.clone())
            .collect::<Vec<AccountMeta>>();
        println!("_in_utxos {:?}", _in_utxos);
        println!("out_utxos {:?}", out_utxos);
        println!("remaining_accounts {:?}", remaining_accounts);
        let inputs_struct = InstructionDataTransfer {
            in_utxos: _in_utxos,
            out_utxos: out_utxos.to_vec(),
            root_indices: root_indices.to_vec(),
            proof: Some(proof.clone()),
            in_tlv_data: in_utxo_tlv_data,
        };
        let mut inputs = Vec::new();
        InstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

        let (cpi_authority_pda, _) = crate::get_cpi_authority_pda();
        let instruction_data = crate::instruction::Transfer { inputs };

        let accounts = crate::accounts::TransferInstruction {
            fee_payer: *fee_payer,
            authority: *authority,
            cpi_authority_pda,
            compressed_pda_program: psp_compressed_pda::ID,
            registered_program_pda: psp_compressed_pda::utils::get_registered_program_pda(
                &psp_compressed_pda::ID,
            ),
            noop_program: NOOP_PROGRAM_ID,
            psp_account_compression_authority: psp_compressed_pda::utils::get_cpi_authority_pda(
                &psp_compressed_pda::ID,
            ),
            account_compression_program: account_compression::ID,
            self_program: crate::ID,
        };

        Instruction {
            program_id: crate::ID,
            accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

            data: instruction_data.data(),
        }
    }
}

#[cfg(test)]
mod test {
    use psp_compressed_pda::{
        tlv::{Tlv, TlvDataElement},
        utxo::{OutUtxo, SerializedUtxos, Utxo},
    };

    use super::*;

    #[test]
    fn test_add_utxos_with_token_tlv_data() {
        let mut serialized_utxos = SerializedUtxos {
            pubkey_array: vec![],
            u64_array: vec![],
            in_utxos: vec![],
            out_utxos: vec![],
        };

        let token_program = Pubkey::new_unique();
        let merkle_tree_pda = Pubkey::new_unique();
        let owner_pubkey = Pubkey::new_unique();
        let mint_pubkey = Pubkey::new_unique();
        let delegate_pubkey = Pubkey::new_unique(); // Assuming there's a delegate for this example
        let accounts = vec![owner_pubkey, mint_pubkey, token_program];
        let merkle_tree_pubkey_0 = Pubkey::new_unique();
        let nullifier_array_pubkey_0 = Pubkey::new_unique();
        let in_utxo_merkle_tree_pubkeys = vec![merkle_tree_pubkey_0];
        let nullifier_array_pubkeys = vec![nullifier_array_pubkey_0];
        let remaing_accounts_pubkeys = vec![merkle_tree_pubkey_0, nullifier_array_pubkey_0];
        // Creating TokenTlvData
        let token_tlv_data = TokenTlvData {
            mint: mint_pubkey,
            owner: owner_pubkey,
            amount: 10_000_000,
            delegate: Some(delegate_pubkey),
            state: AccountState::Initialized,
            is_native: None,
            delegated_amount: 5000,
        };

        // Assuming we have a way to serialize TokenTlvData to Vec<u8>
        let token_data_serialized = token_tlv_data.try_to_vec().unwrap();
        let token_data_hash = token_tlv_data.hash().unwrap(); // Assuming hash() returns a Result

        // Creating TLV data element with TokenTlvData
        let token_tlv_data_element = TlvDataElement {
            discriminator: 2u64.to_le_bytes(),
            owner: token_program,
            data: token_data_serialized,
            data_hash: token_data_hash,
        };

        let tlv_data = Tlv {
            tlv_elements: vec![token_tlv_data_element],
        };

        // Convert TLV data to a serializable format
        let mut pubkey_array_for_tlv = Vec::new();
        let tlv_serializable = tlv_data.to_serializable_tlv(&mut pubkey_array_for_tlv, &accounts);

        let mut utxo = Utxo {
            owner: owner_pubkey,
            blinding: [0u8; 32],
            lamports: 100,
            data: Some(tlv_data.clone()),
        };
        let leaf_index = 1u32;
        utxo.update_blinding(merkle_tree_pda, (leaf_index as usize).clone())
            .unwrap();

        // Assuming add_in_utxos is modified to accept UTXOs with TLV data correctly
        serialized_utxos
            .add_in_utxos(
                &[utxo.clone()],
                &accounts,
                &[leaf_index],
                &in_utxo_merkle_tree_pubkeys,
                &nullifier_array_pubkeys,
            )
            .unwrap();

        // Create OutUtxo
        let out_utxo = OutUtxo {
            owner: owner_pubkey,
            lamports: 100,
            data: Some(tlv_data),
        };

        // Add OutUtxo
        serialized_utxos
            .add_out_utxos(
                &[out_utxo.clone()],
                &accounts,
                &remaing_accounts_pubkeys,
                &[merkle_tree_pubkey_0],
            )
            .unwrap();

        assert_eq!(
            serialized_utxos.in_utxos.len(),
            1,
            "Should have added one UTXO with TLV data"
        );
        assert!(
            serialized_utxos.in_utxos[0]
                .in_utxo_serializable
                .data
                .is_some(),
            "UTXO should contain TLV data"
        );
        assert_eq!(
            serialized_utxos.out_utxos.len(),
            1,
            "Should have added one out UTXO with TLV data"
        );
        assert!(
            serialized_utxos.out_utxos[0]
                .out_utxo_serializable
                .data
                .is_some(),
            "UTXO should contain TLV data"
        );
        // Verify that TLV data was serialized correctly
        let serialized_tlv_data = serialized_utxos.in_utxos[0]
            .in_utxo_serializable
            .data
            .as_ref()
            .unwrap();
        assert_eq!(
            *serialized_tlv_data, tlv_serializable,
            "Serialized TLV data should match the expected serialized version"
        );
        let deserialized_in_utxos = serialized_utxos
            .in_utxos_from_serialized_utxos(&accounts, &[merkle_tree_pda])
            .unwrap();
        assert_eq!(deserialized_in_utxos[0].in_utxo, utxo);

        let deserialized_out_utxos = serialized_utxos
            .out_utxos_from_serialized_utxos(&accounts)
            .unwrap();
        assert_eq!(deserialized_out_utxos[0].out_utxo, out_utxo);
    }
}

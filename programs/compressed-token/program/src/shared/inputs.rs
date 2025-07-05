// TODO: use in get inputs.
pub fn add_data_hash_to_input_compressed_accounts<const FROZEN_INPUTS: bool>(
    input_compressed_accounts_with_merkle_context: &mut [InAccount],
    input_token_data: &[TokenData],
    hashed_mint: &[u8; 32],
    remaining_accounts: &[AccountInfo<'_>],
) -> Result<()> {
    for (i, compressed_account_with_context) in input_compressed_accounts_with_merkle_context
        .iter_mut()
        .enumerate()
    {
        let hashed_owner = hash_to_bn254_field_size_be(&input_token_data[i].owner.to_bytes());

        let mut amount_bytes = [0u8; 32];
        let discriminator_bytes = &remaining_accounts[compressed_account_with_context
            .merkle_context
            .merkle_tree_pubkey_index
            as usize]
            .try_borrow_data()?[0..8];
        match discriminator_bytes {
            StateMerkleTreeAccount::DISCRIMINATOR => {
                amount_bytes[24..]
                    .copy_from_slice(input_token_data[i].amount.to_le_bytes().as_slice());
                Ok(())
            }
            BATCHED_DISCRIMINATOR => {
                amount_bytes[24..]
                    .copy_from_slice(input_token_data[i].amount.to_be_bytes().as_slice());
                Ok(())
            }
            OUTPUT_QUEUE_DISCRIMINATOR => {
                amount_bytes[24..]
                    .copy_from_slice(input_token_data[i].amount.to_be_bytes().as_slice());
                Ok(())
            }
            _ => {
                msg!(
                    "{} is no Merkle tree or output queue account. ",
                    remaining_accounts[compressed_account_with_context
                        .merkle_context
                        .merkle_tree_pubkey_index as usize]
                        .key()
                );
                err!(anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch)
            }
        }?;
        let delegate_store;
        let hashed_delegate = if let Some(delegate) = input_token_data[i].delegate {
            delegate_store = hash_to_bn254_field_size_be(&delegate.to_bytes());
            Some(&delegate_store)
        } else {
            None
        };
        compressed_account_with_context.data_hash = if !FROZEN_INPUTS {
            TokenData::hash_with_hashed_values(
                hashed_mint,
                &hashed_owner,
                &amount_bytes,
                &hashed_delegate,
            )
            .map_err(ProgramError::from)?
        } else {
            TokenData::hash_frozen_with_hashed_values(
                hashed_mint,
                &hashed_owner,
                &amount_bytes,
                &hashed_delegate,
            )
            .map_err(ProgramError::from)?
        };
    }
    Ok(())
}

pub fn get_input_compressed_accounts_with_merkle_context_and_check_signer<const IS_FROZEN: bool>(
    signer: &Pubkey,
    signer_is_delegate: &Option<DelegatedTransfer>,
    remaining_accounts: &[AccountInfo<'_>],
    input_token_data_with_context: &[InputTokenDataWithContext],
    mint: &Pubkey,
) -> Result<(Vec<InAccount>, Vec<TokenData>, u64)> {
    // Collect the total number of lamports to check whether inputs and outputs
    // are unbalanced. If unbalanced create a non token compressed change
    // account owner by the sender.
    let mut sum_lamports = 0;
    let mut input_compressed_accounts_with_merkle_context: Vec<InAccount> =
        Vec::<InAccount>::with_capacity(input_token_data_with_context.len());
    let mut input_token_data_vec: Vec<TokenData> =
        Vec::with_capacity(input_token_data_with_context.len());

    for input_token_data in input_token_data_with_context.iter() {
        let owner = if input_token_data.delegate_index.is_none() {
            *signer
        } else if let Some(signer_is_delegate) = signer_is_delegate {
            signer_is_delegate.owner
        } else {
            *signer
        };
        // This is a check for convenience to throw a meaningful error.
        // The actual security results from the proof verification.
        if signer_is_delegate.is_some()
            && input_token_data.delegate_index.is_some()
            && *signer
                != remaining_accounts[input_token_data.delegate_index.unwrap() as usize].key()
        {
            msg!(
                "signer {:?} != delegate in remaining accounts {:?}",
                signer,
                remaining_accounts[input_token_data.delegate_index.unwrap() as usize].key()
            );
            msg!(
                "delegate index {:?}",
                input_token_data.delegate_index.unwrap() as usize
            );
            return err!(ErrorCode::DelegateSignerCheckFailed);
        }

        let compressed_account = InAccount {
            lamports: input_token_data.lamports.unwrap_or_default(),
            discriminator: TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
            merkle_context: input_token_data.merkle_context,
            root_index: input_token_data.root_index,
            data_hash: [0u8; 32],
            address: None,
        };
        sum_lamports += compressed_account.lamports;
        let state = if IS_FROZEN {
            AccountState::Frozen
        } else {
            AccountState::Initialized
        };
        if input_token_data.tlv.is_some() {
            unimplemented!("Tlv is unimplemented.");
        }
        let token_data = TokenData {
            mint: *mint,
            owner,
            amount: input_token_data.amount,
            delegate: input_token_data.delegate_index.map(|_| {
                remaining_accounts[input_token_data.delegate_index.unwrap() as usize].key()
            }),
            state,
            tlv: None,
        };
        input_token_data_vec.push(token_data);
        input_compressed_accounts_with_merkle_context.push(compressed_account);
    }
    Ok((
        input_compressed_accounts_with_merkle_context,
        input_token_data_vec,
        sum_lamports,
    ))
}

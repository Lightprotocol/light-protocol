/// Creates output compressed accounts.
/// Steps:
/// 1. Allocate memory for token data.
/// 2. Create, hash and serialize token data.
/// 3. Create compressed account data.
/// 4. Repeat for every pubkey.
#[allow(clippy::too_many_arguments)]
pub fn create_output_compressed_accounts(
    output_compressed_accounts: &mut [OutputCompressedAccountWithPackedContext],
    mint_pubkey: impl AsPubkey,
    pubkeys: &[impl AsPubkey],
    delegate: Option<Pubkey>,
    is_delegate: Option<Vec<bool>>,
    amounts: &[impl ZeroCopyNumTrait],
    lamports: Option<Vec<Option<impl ZeroCopyNumTrait>>>,
    hashed_mint: &[u8; 32],
) -> Result<u64> {
    let mut sum_lamports = 0;
    let hashed_delegate_store = if let Some(delegate) = delegate {
        hash_to_bn254_field_size_be(delegate.to_bytes().as_slice())
    } else {
        [0u8; 32]
    };
    for (i, (owner, amount)) in pubkeys.iter().zip(amounts.iter()).enumerate() {
        let (delegate, hashed_delegate) = if is_delegate
            .as_ref()
            .map(|is_delegate| is_delegate[i])
            .unwrap_or(false)
        {
            (
                delegate.as_ref().map(|delegate_pubkey| *delegate_pubkey),
                Some(&hashed_delegate_store),
            )
        } else {
            (None, None)
        };
        // 107/75 =
        //      32      mint
        // +    32      owner
        // +    8       amount
        // +    1 + 32  option + delegate (optional)
        // +    1       state
        // +    1       tlv (None)
        let capacity = if delegate.is_some() { 107 } else { 75 };
        let mut token_data_bytes = Vec::with_capacity(capacity);
        // 1,000 CU token data and serialize
        let token_data = TokenData {
            mint: (mint_pubkey).to_anchor_pubkey(),
            owner: (*owner).to_anchor_pubkey(),
            amount: (*amount).into(),
            delegate,
            state: AccountState::Initialized,
            tlv: None,
        };
        // TODO: remove serialization, just write bytes.
        token_data.serialize(&mut token_data_bytes).unwrap();
        bench_sbf_start!("token_data_hash");
        let hashed_owner = hash_to_bn254_field_size_be(owner.to_pubkey_bytes().as_slice());

        let mut amount_bytes = [0u8; 32];
        amount_bytes[24..].copy_from_slice(amount.to_bytes_be().as_slice());

        let data_hash = TokenData::hash_with_hashed_values(
            hashed_mint,
            &hashed_owner,
            &amount_bytes,
            &hashed_delegate,
        )
        .map_err(ProgramError::from)?;
        let data = CompressedAccountData {
            discriminator: TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
            data: token_data_bytes,
            data_hash,
        };

        bench_sbf_end!("token_data_hash");
        let lamports = lamports
            .as_ref()
            .and_then(|lamports| lamports[i])
            .unwrap_or(0u64.into());
        sum_lamports += lamports.into();
        output_compressed_accounts[i] = OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                owner: crate::ID.into(),
                lamports: lamports.into(),
                data: Some(data),
                address: None,
            },
            merkle_tree_index: merkle_tree_indices[i],
        };
    }
    Ok(sum_lamports)
}

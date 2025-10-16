use light_client::{
    indexer::{CompressedTokenAccount, Indexer},
    rpc::Rpc,
};
use light_compressed_token_sdk::{
    account2::CTokenAccount2,
    error::TokenSdkError,
    instructions::transfer2::{
        account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction, Transfer2Config,
        Transfer2Inputs,
    },
    token_pool::find_token_pool_pda_with_index,
};
use light_ctoken_types::{
    instructions::transfer2::{MultiInputTokenDataWithContext, MultiTokenTransferOutputData},
    state::TokenDataVersion,
    COMPRESSED_TOKEN_PROGRAM_ID,
};
use light_sdk::instruction::{PackedAccounts, PackedStateTreeInfo};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

pub fn pack_input_token_account(
    account: &CompressedTokenAccount,
    tree_info: &PackedStateTreeInfo,
    packed_accounts: &mut PackedAccounts,
    in_lamports: &mut Vec<u64>,
    is_delegate_transfer: bool, // Explicitly specify if delegate is signing
    token_data_version: TokenDataVersion,
) -> MultiInputTokenDataWithContext {
    // Check if account has a delegate
    let has_delegate = account.token.delegate.is_some();

    // Determine who should be the signer
    // For delegate transfers, the account MUST have a delegate set
    // If is_delegate_transfer is true but no delegate exists, owner must sign
    let owner_is_signer = !is_delegate_transfer || !has_delegate;

    let delegate_index = if let Some(delegate) = account.token.delegate {
        // Delegate is signer only if this is explicitly a delegate transfer
        packed_accounts.insert_or_get_config(delegate, is_delegate_transfer, false)
    } else {
        0
    };

    if account.account.lamports != 0 {
        in_lamports.push(account.account.lamports);
    }

    MultiInputTokenDataWithContext {
        amount: account.token.amount,
        merkle_context: light_compressed_account::compressed_account::PackedMerkleContext {
            merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
            queue_pubkey_index: tree_info.queue_pubkey_index,
            leaf_index: tree_info.leaf_index,
            prove_by_index: tree_info.prove_by_index,
        },
        root_index: tree_info.root_index,
        mint: packed_accounts.insert_or_get_read_only(account.token.mint),
        owner: packed_accounts.insert_or_get_config(account.token.owner, owner_is_signer, false),
        has_delegate, // Indicates if account has a delegate set
        delegate: delegate_index,
        version: token_data_version as u8, // V2 for batched Merkle trees
    }
}

pub async fn create_decompress_instruction<R: Rpc + Indexer>(
    rpc: &mut R,
    compressed_token_account: &[CompressedTokenAccount],
    decompress_amount: u64,
    solana_token_account: Pubkey,
    payer: Pubkey,
) -> Result<Instruction, TokenSdkError> {
    create_generic_transfer2_instruction(
        rpc,
        vec![Transfer2InstructionType::Decompress(DecompressInput {
            compressed_token_account: compressed_token_account.to_vec(),
            decompress_amount,
            solana_token_account,
            amount: decompress_amount,
            pool_index: None,
        })],
        payer,
        false,
    )
    .await
}
#[derive(Debug, Clone, PartialEq)]
pub struct TransferInput {
    pub compressed_token_account: Vec<CompressedTokenAccount>,
    pub to: Pubkey,
    pub amount: u64,
    pub is_delegate_transfer: bool, // Indicates if delegate is the signer
    pub mint: Option<Pubkey>,       // Required when compressed_token_account is empty
    pub change_amount: Option<u64>, // Optional: explicitly set change amount to keep
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecompressInput {
    pub compressed_token_account: Vec<CompressedTokenAccount>,
    pub decompress_amount: u64,
    pub solana_token_account: Pubkey,
    pub amount: u64,
    pub pool_index: Option<u8>, // For SPL only. None = default (0), Some(n) = specific pool
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompressInput {
    pub compressed_token_account: Option<Vec<CompressedTokenAccount>>,
    pub solana_token_account: Pubkey,
    pub to: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub authority: Pubkey,
    pub output_queue: Pubkey,
    pub pool_index: Option<u8>, // For SPL only. None = default (0), Some(n) = specific pool
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompressAndCloseInput {
    pub solana_ctoken_account: Pubkey,
    pub authority: Pubkey,
    pub output_queue: Pubkey,
    pub destination: Option<Pubkey>,
    pub is_compressible: bool, // If true, account has extensions; if false, regular CToken ATA
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApproveInput {
    pub compressed_token_account: Vec<CompressedTokenAccount>,
    pub delegate: Pubkey,
    pub delegate_amount: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Transfer2InstructionType {
    Compress(CompressInput),
    Decompress(DecompressInput),
    Transfer(TransferInput),
    Approve(ApproveInput),
    CompressAndClose(CompressAndCloseInput),
}

// Note doesn't support multiple signers.
pub async fn create_generic_transfer2_instruction<R: Rpc + Indexer>(
    rpc: &mut R,
    actions: Vec<Transfer2InstructionType>,
    payer: Pubkey,
    should_filter_zero_outputs: bool,
) -> Result<Instruction, TokenSdkError> {
    println!("here");
    // // Get a single shared output queue for ALL compress/compress-and-close operations
    // // This prevents reordering issues caused by the sort_by_key at the end
    // let shared_output_queue = rpc
    //     .get_random_state_tree_info()
    //     .unwrap()
    //     .get_output_pubkey()
    //     .unwrap();

    let mut hashes = Vec::new();
    actions.iter().for_each(|account| match account {
        Transfer2InstructionType::Compress(input) => {
            // Also collect hashes from compressed inputs if present
            if let Some(ref compressed_accounts) = input.compressed_token_account {
                compressed_accounts
                    .iter()
                    .for_each(|account| hashes.push(account.account.hash));
            }
        }
        Transfer2InstructionType::CompressAndClose(_) => {
            // CompressAndClose doesn't have compressed inputs, only Solana CToken account
        }
        Transfer2InstructionType::Decompress(input) => input
            .compressed_token_account
            .iter()
            .for_each(|account| hashes.push(account.account.hash)),
        Transfer2InstructionType::Transfer(input) => input
            .compressed_token_account
            .iter()
            .for_each(|account| hashes.push(account.account.hash)),
        Transfer2InstructionType::Approve(input) => input
            .compressed_token_account
            .iter()
            .for_each(|account| hashes.push(account.account.hash)),
    });
    let rpc_proof_result = rpc
        .get_validity_proof(hashes, vec![], None)
        .await
        .unwrap()
        .value;

    let mut packed_tree_accounts = PackedAccounts::default();
    // tree infos must be packed before packing the token input accounts
    let packed_tree_infos = rpc_proof_result.pack_tree_infos(&mut packed_tree_accounts);

    // We use a single shared output queue for all compress/compress-and-close operations to avoid ordering failures.
    let shared_output_queue = if packed_tree_infos.address_trees.is_empty() {
        let shared_output_queue = rpc
            .get_random_state_tree_info()
            .unwrap()
            .get_output_pubkey()
            .unwrap();
        packed_tree_accounts.insert_or_get(shared_output_queue)
    } else {
        packed_tree_infos
            .state_trees
            .as_ref()
            .unwrap()
            .output_tree_index
    };

    let mut inputs_offset = 0;
    let mut in_lamports = Vec::new();
    let mut out_lamports = Vec::new();
    let mut token_accounts = Vec::new();
    for action in actions {
        match action {
            Transfer2InstructionType::Compress(input) => {
                let mut token_account =
                    if let Some(ref input_token_account) = input.compressed_token_account {
                        let token_data = input_token_account
                            .iter()
                            .zip(
                                packed_tree_infos
                                    .state_trees
                                    .as_ref()
                                    .unwrap()
                                    .packed_tree_infos[inputs_offset..]
                                    .iter(),
                            )
                            .map(|(account, rpc_account)| {
                                if input.to != account.token.owner {
                                    return Err(TokenSdkError::InvalidCompressInputOwner);
                                }
                                Ok(pack_input_token_account(
                                    account,
                                    rpc_account,
                                    &mut packed_tree_accounts,
                                    &mut in_lamports,
                                    false, // Compress is always owner-signed
                                    TokenDataVersion::from_discriminator(
                                        account.account.data.as_ref().unwrap().discriminator,
                                    )
                                    .unwrap(),
                                ))
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        inputs_offset += token_data.len();
                        CTokenAccount2::new(token_data)?
                    } else {
                        CTokenAccount2::new_empty(
                            packed_tree_accounts.insert_or_get(input.to),
                            packed_tree_accounts.insert_or_get(input.mint),
                        )
                    };

                let source_index = packed_tree_accounts.insert_or_get(input.solana_token_account);
                let authority_index =
                    packed_tree_accounts.insert_or_get_config(input.authority, true, false);

                // Check if source account is an SPL token account
                let source_account_owner = rpc
                    .get_account(input.solana_token_account)
                    .await
                    .unwrap()
                    .unwrap()
                    .owner;

                if source_account_owner.to_bytes() != COMPRESSED_TOKEN_PROGRAM_ID {
                    // For SPL compression, get mint first
                    let mint = input.mint;

                    // Add the SPL Token program that owns the account
                    let _token_program_index =
                        packed_tree_accounts.insert_or_get_read_only(source_account_owner);

                    // Use pool_index from input, default to 0
                    let pool_index = input.pool_index.unwrap_or(0);
                    let (token_pool_pda, bump) = find_token_pool_pda_with_index(&mint, pool_index);
                    let pool_account_index = packed_tree_accounts.insert_or_get(token_pool_pda);

                    // Use the new SPL-specific compress method
                    token_account.compress_spl(
                        input.amount,
                        source_index,
                        authority_index,
                        pool_account_index,
                        pool_index,
                        bump,
                    )?;
                } else {
                    // Regular compression for compressed token accounts
                    token_account.compress_ctoken(input.amount, source_index, authority_index)?;
                }
                token_accounts.push(token_account);
            }
            Transfer2InstructionType::Decompress(input) => {
                let token_data = input
                    .compressed_token_account
                    .iter()
                    .zip(
                        packed_tree_infos
                            .state_trees
                            .as_ref()
                            .unwrap()
                            .packed_tree_infos[inputs_offset..]
                            .iter(),
                    )
                    .map(|(account, rpc_account)| {
                        pack_input_token_account(
                            account,
                            rpc_account,
                            &mut packed_tree_accounts,
                            &mut in_lamports,
                            false, // Decompress is always owner-signed
                            TokenDataVersion::from_discriminator(
                                account.account.data.as_ref().unwrap().discriminator,
                            )
                            .unwrap(),
                        )
                    })
                    .collect::<Vec<_>>();
                inputs_offset += token_data.len();
                let mut token_account = CTokenAccount2::new(token_data)?;
                // Add recipient SPL token account
                let recipient_index =
                    packed_tree_accounts.insert_or_get(input.solana_token_account);
                let recipient_account_owner = rpc
                    .get_account(input.solana_token_account)
                    .await
                    .unwrap()
                    .unwrap()
                    .owner;

                if recipient_account_owner.to_bytes() != COMPRESSED_TOKEN_PROGRAM_ID {
                    // For SPL decompression, get mint first
                    let mint = input.compressed_token_account[0].token.mint;

                    // Add the SPL Token program that owns the account
                    let _token_program_index =
                        packed_tree_accounts.insert_or_get_read_only(recipient_account_owner);

                    // Use pool_index from input, default to 0
                    let pool_index = input.pool_index.unwrap_or(0);
                    let (token_pool_pda, bump) = find_token_pool_pda_with_index(&mint, pool_index);
                    let pool_account_index = packed_tree_accounts.insert_or_get(token_pool_pda);

                    // Use the new SPL-specific decompress method
                    token_account.decompress_spl(
                        input.decompress_amount,
                        recipient_index,
                        pool_account_index,
                        pool_index,
                        bump,
                    )?;
                } else {
                    // Use the new SPL-specific decompress method
                    token_account.decompress_ctoken(input.decompress_amount, recipient_index)?;
                }

                out_lamports.push(
                    input
                        .compressed_token_account
                        .iter()
                        .map(|account| account.account.lamports)
                        .sum::<u64>(),
                );

                token_accounts.push(token_account);
            }
            Transfer2InstructionType::Transfer(input) => {
                println!("here1");
                let token_data = input
                    .compressed_token_account
                    .iter()
                    .zip(
                        packed_tree_infos
                            .state_trees
                            .as_ref()
                            .unwrap()
                            .packed_tree_infos[inputs_offset..]
                            .iter(),
                    )
                    .map(|(account, rpc_account)| {
                        pack_input_token_account(
                            account,
                            rpc_account,
                            &mut packed_tree_accounts,
                            &mut in_lamports,
                            input.is_delegate_transfer, // Use the flag from TransferInput
                            TokenDataVersion::from_discriminator(
                                account.account.data.as_ref().unwrap().discriminator,
                            )
                            .unwrap(),
                        )
                    })
                    .collect::<Vec<_>>();
                println!("here2 {:?}", token_data);
                inputs_offset += token_data.len();
                if token_data.is_empty() {
                    // When no input accounts, create recipient account directly
                    // This requires mint to be specified in the input
                    let mint = input.mint.ok_or(TokenSdkError::InvalidAccountData)?;

                    let recipient_index = packed_tree_accounts.insert_or_get(input.to);
                    let mint_index = packed_tree_accounts.insert_or_get_read_only(mint);

                    let recipient_token_account = CTokenAccount2 {
                        inputs: vec![],
                        output: MultiTokenTransferOutputData {
                            owner: recipient_index,
                            amount: input.amount,
                            has_delegate: false,
                            delegate: 0,
                            mint: mint_index,
                            version: TokenDataVersion::V2 as u8, // Default to V2
                        },
                        compression: None,
                        delegate_is_set: false,
                        method_used: true, // Mark that this account was used/created
                    };

                    out_lamports.push(0);
                    token_accounts.push(recipient_token_account);
                } else {
                    // Only use new_delegated if the input accounts actually have delegates
                    let has_delegates = token_data.iter().any(|data| data.has_delegate);
                    println!(
                        "is_delegate_transfer: {}, has_delegates: {}",
                        input.is_delegate_transfer, has_delegates
                    );
                    let mut token_account = if input.is_delegate_transfer && has_delegates {
                        CTokenAccount2::new_delegated(token_data)
                    } else {
                        CTokenAccount2::new(token_data)
                    }?;
                    let recipient_index = packed_tree_accounts.insert_or_get(input.to);
                    let recipient_token_account =
                        token_account.transfer(recipient_index, input.amount)?;
                    if let Some(amount) = input.change_amount {
                        token_account.output.amount = amount;
                    }
                    // all lamports go to the sender.
                    out_lamports.push(
                        input
                            .compressed_token_account
                            .iter()
                            .map(|account| account.account.lamports)
                            .sum::<u64>(),
                    );
                    // For consistency add 0 lamports for the recipient.
                    out_lamports.push(0);
                    token_accounts.push(token_account);
                    token_accounts.push(recipient_token_account);
                }
            }
            Transfer2InstructionType::Approve(input) => {
                let token_data = input
                    .compressed_token_account
                    .iter()
                    .zip(
                        packed_tree_infos
                            .state_trees
                            .as_ref()
                            .unwrap()
                            .packed_tree_infos[inputs_offset..]
                            .iter(),
                    )
                    .map(|(account, rpc_account)| {
                        pack_input_token_account(
                            account,
                            rpc_account,
                            &mut packed_tree_accounts,
                            &mut in_lamports,
                            false, // Approve is always owner-signed
                            TokenDataVersion::from_discriminator(
                                account.account.data.as_ref().unwrap().discriminator,
                            )
                            .unwrap(),
                        )
                    })
                    .collect::<Vec<_>>();
                inputs_offset += token_data.len();
                let mut token_account = CTokenAccount2::new(token_data)?;
                let delegate_index = packed_tree_accounts.insert_or_get(input.delegate);
                let delegated_token_account =
                    token_account.approve(delegate_index, input.delegate_amount)?;
                // all lamports stay with the owner
                out_lamports.push(
                    input
                        .compressed_token_account
                        .iter()
                        .map(|account| account.account.lamports)
                        .sum::<u64>(),
                );
                out_lamports.push(0);
                // For consistency add 0 lamports for the delegated account
                token_accounts.push(token_account);
                token_accounts.push(delegated_token_account);
            }
            Transfer2InstructionType::CompressAndClose(input) => {
                println!(
                    "input.solana_ctoken_account {:?}",
                    input.solana_ctoken_account
                );
                // Get token account info to extract mint, balance, owner, and rent_sponsor
                let token_account_info = rpc
                    .get_account(input.solana_ctoken_account)
                    .await
                    .map_err(|_| TokenSdkError::InvalidAccountData)?
                    .ok_or(TokenSdkError::InvalidAccountData)?;

                // Parse the compressed token account using zero-copy deserialization
                use light_ctoken_types::state::{CToken, ZExtensionStruct};
                use light_zero_copy::traits::ZeroCopyAt;
                let (compressed_token, _) = CToken::zero_copy_at(&token_account_info.data)
                    .map_err(|_| TokenSdkError::InvalidAccountData)?;
                println!("compressed_token {:?}", compressed_token);
                let mint = compressed_token.mint;
                let balance = compressed_token.amount;
                let owner = compressed_token.owner;

                // Extract rent_sponsor, compression_authority, and compress_to_pubkey from compressible extension
                // For non-compressible accounts, use the owner as the rent_sponsor
                let (rent_sponsor, _compression_authority, compress_to_pubkey) = if input
                    .is_compressible
                {
                    if let Some(extensions) = compressed_token.extensions.as_ref() {
                        let mut found_rent_sponsor = None;
                        let mut found_compression_authority = None;
                        let mut found_compress_to_pubkey = false;
                        for extension in extensions {
                            if let ZExtensionStruct::Compressible(compressible_ext) = extension {
                                found_rent_sponsor = Some(compressible_ext.rent_sponsor);
                                found_compression_authority =
                                    Some(compressible_ext.compression_authority);
                                found_compress_to_pubkey = compressible_ext.compress_to_pubkey == 1;
                                break;
                            }
                        }
                        println!("rent sponsor {:?}", found_rent_sponsor);
                        println!("compress_to_pubkey {:?}", found_compress_to_pubkey);
                        (
                            found_rent_sponsor.ok_or(TokenSdkError::InvalidAccountData)?,
                            found_compression_authority,
                            found_compress_to_pubkey,
                        )
                    } else {
                        println!("no extensions but is_compressible is true");
                        return Err(TokenSdkError::InvalidAccountData);
                    }
                } else {
                    // Non-compressible account: use owner as rent_sponsor
                    println!("non-compressible account, using owner as rent sponsor");
                    (owner.to_bytes(), None, false)
                };

                // Add source account first (it's being closed, so needs to be writable)
                let source_index = packed_tree_accounts.insert_or_get(input.solana_ctoken_account);

                // Determine the owner index for the compressed output
                // If compress_to_pubkey is true, reuse source_index; otherwise add original owner
                let owner_index = if compress_to_pubkey {
                    source_index // Reuse the source account index as owner
                } else {
                    packed_tree_accounts.insert_or_get(Pubkey::from(owner.to_bytes()))
                };

                let mint_index =
                    packed_tree_accounts.insert_or_get_read_only(Pubkey::from(mint.to_bytes()));
                let rent_sponsor_index =
                    packed_tree_accounts.insert_or_get(Pubkey::from(rent_sponsor));

                // Create token account with the full balance
                let mut token_account = CTokenAccount2::new_empty(owner_index, mint_index);
                // Authority needs to be writable if it's also the destination (receives lamports from close)
                let authority_needs_writable = input.destination.is_none();
                let authority_index = packed_tree_accounts.insert_or_get_config(
                    input.authority,
                    true,
                    authority_needs_writable,
                );

                // Use compress_and_close method with the actual balance
                // The compressed_account_index should match the position in token_accounts
                // Destination always receives the compression incentive (11k lamports)
                let destination_index = input
                    .destination
                    .map(|d| packed_tree_accounts.insert_or_get(d))
                    .unwrap_or(authority_index); // Default to authority if no destination specified

                token_account.compress_and_close(
                    (*balance).into(),
                    source_index,
                    authority_index,
                    rent_sponsor_index,         // Use the extracted rent_sponsor
                    token_accounts.len() as u8, // Index in the output array
                    destination_index,
                )?;

                token_accounts.push(token_account);
            }
        }
    }

    // // Sort token accounts by merkle_tree index to ensure OutputMerkleTreeIndicesNotInOrder error doesn't occur
    // // The system program requires output merkle tree indices to be in ascending order
    // token_accounts.sort_by_key(|account| account.output.merkle_tree);
    let transfer_config = if should_filter_zero_outputs {
        Transfer2Config::default().filter_zero_amount_outputs()
    } else {
        Transfer2Config::default()
    };
    let packed_accounts = packed_tree_accounts.to_account_metas().0;
    let inputs = Transfer2Inputs {
        validity_proof: rpc_proof_result.proof,
        transfer_config,
        meta_config: Transfer2AccountsMetaConfig {
            fee_payer: Some(payer),
            packed_accounts: Some(packed_accounts),
            ..Default::default()
        },
        in_lamports: if in_lamports.is_empty() {
            None
        } else {
            Some(in_lamports)
        },
        out_lamports: if out_lamports.iter().all(|lamports| *lamports == 0) {
            None
        } else {
            Some(out_lamports)
        },
        token_accounts,
        output_queue: shared_output_queue,
    };
    println!("pre create_transfer2_instruction {:?}", inputs);
    create_transfer2_instruction(inputs)
}

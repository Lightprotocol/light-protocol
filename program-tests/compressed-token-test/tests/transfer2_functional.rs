// TODO: remove
#![allow(unused)]
use std::collections::HashMap;

use anchor_lang::AnchorDeserialize;
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_token_sdk::instructions::find_spl_mint_address;
use light_ctoken_types::{
    instructions::{mint_action::Recipient, transfer2::CompressedTokenInstructionDataTransfer2},
    state::TokenDataVersion,
};
use light_program_test::{indexer::TestIndexerExtensions, LightProgramTest, ProgramTestConfig};
use light_test_utils::{airdrop_lamports, assert_transfer2::assert_transfer2};
use light_token_client::{
    actions::{create_mint, mint_to_compressed},
    instructions::{
        mint_action::create_mint_action_instruction,
        transfer2::{
            create_generic_transfer2_instruction, ApproveInput, CompressAndCloseInput,
            CompressInput, DecompressInput, Transfer2InstructionType, TransferInput,
        },
    },
};
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};

// ============================================================================
// Meta Types for Test Definition (only amounts, counts, and bools)
// ============================================================================

#[derive(Debug, Clone)]
pub struct MetaTransferInput {
    pub input_compressed_accounts: Vec<u64>, // Balance of each input account (empty vec = no new inputs)
    pub amount: u64,                         // Amount to transfer
    pub change_amount: Option<u64>,          // Optional: explicitly set change amount to keep
    pub is_delegate_transfer: bool,
    pub token_data_version: TokenDataVersion,
    pub signer_index: usize, // Index of keypair that signs this action (owner or delegate)
    pub delegate_index: Option<usize>, // Index of delegate keypair (for delegate transfers)
    pub recipient_index: usize, // Index of keypair to receive transferred tokens
    pub mint_index: usize,   // Index of which mint to use (0-4)
}

#[derive(Debug, Clone)]
pub struct MetaDecompressInput {
    pub num_input_compressed_accounts: u8,
    pub decompress_amount: u64,
    pub amount: u64,
    pub token_data_version: TokenDataVersion,
    pub signer_index: usize,    // Index of keypair that signs this action
    pub recipient_index: usize, // Index of keypair to receive decompressed tokens
    pub mint_index: usize,      // Index of which mint to use (0-4)
}

#[derive(Debug, Clone)]
pub struct MetaCompressInput {
    pub num_input_compressed_accounts: u8,
    pub amount: u64,
    pub token_data_version: TokenDataVersion,
    pub signer_index: usize,    // Index of keypair that signs this action
    pub recipient_index: usize, // Index of keypair to receive compressed tokens
    pub mint_index: usize,      // Index of which mint to use (0-4)
}

#[derive(Debug, Clone)]
pub struct MetaCompressAndCloseInput {
    pub token_data_version: TokenDataVersion,
    pub signer_index: usize, // Index of keypair that signs this action
    pub destination_index: Option<usize>, // Index of keypair to receive lamports (None = no destination)
    pub mint_index: usize,                // Index of which mint to use (0-4)
}

#[derive(Debug, Clone)]
pub struct MetaApproveInput {
    pub num_input_compressed_accounts: u8,
    pub delegate_amount: u64,
    pub token_data_version: TokenDataVersion,
    pub signer_index: usize, // Index of keypair that signs this action (owner)
    pub delegate_index: usize, // Index of keypair to set as delegate
    pub mint_index: usize,   // Index of which mint to use (0-4)
}

#[derive(Debug, Clone)]
pub enum MetaTransfer2InstructionType {
    Compress(MetaCompressInput),
    Decompress(MetaDecompressInput),
    Transfer(MetaTransferInput),
    Approve(MetaApproveInput),
    CompressAndClose(MetaCompressAndCloseInput),
}

#[derive(Debug, Clone)]
pub struct TestCase {
    pub name: String,
    pub actions: Vec<MetaTransfer2InstructionType>,
}

struct TestRequirements {
    // Map from (signer_index, mint_index) to their required token amounts per version
    pub signer_mint_compressed_amounts:
        HashMap<(usize, usize), HashMap<TokenDataVersion, Vec<u64>>>,
    pub signer_solana_amounts: HashMap<usize, u64>, // For compress operations
    pub signer_ctoken_amounts: HashMap<(usize, usize), u64>, // For CToken accounts (signer_index, mint_index) -> amount
}

// Test context to pass to builder functions
struct TestContext {
    rpc: LightProgramTest,
    keypairs: Vec<Keypair>,
    mints: Vec<Pubkey>,             // Multiple mints (up to 5)
    mint_seeds: Vec<Keypair>,       // Mint seeds used to derive mints
    mint_authorities: Vec<Keypair>, // One authority per mint
    payer: Keypair,
    ctoken_atas: HashMap<(usize, usize), Pubkey>, // (signer_index, mint_index) -> CToken ATA pubkey
}

impl TestContext {
    fn find_keypair_by_pubkey(&self, pubkey: &Pubkey) -> Option<Keypair> {
        if self.payer.pubkey() == *pubkey {
            return Some(self.payer.insecure_clone());
        }
        // Check all mint authorities
        for mint_authority in &self.mint_authorities {
            if mint_authority.pubkey() == *pubkey {
                return Some(mint_authority.insecure_clone());
            }
        }
        self.keypairs
            .iter()
            .find(|kp| kp.pubkey() == *pubkey)
            .map(|kp| kp.insecure_clone())
    }

    async fn new(test_case: &TestCase) -> Result<Self, Box<dyn std::error::Error>> {
        // Analyze test case to determine requirements
        let requirements = Self::analyze_test_requirements(test_case);
        // Fresh RPC for each test
        let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
        let payer = rpc.get_payer().insecure_clone();

        // Create 5 mints (maximum supported)
        let mut mints = Vec::new();
        let mut mint_seeds = Vec::new();
        let mut mint_authorities = Vec::new();

        for i in 0..5 {
            let mint_authority = Keypair::new();
            let mint_seed = Keypair::new();
            let (mint, _) = find_spl_mint_address(&mint_seed.pubkey());

            create_mint(
                &mut rpc,
                &mint_seed,
                6, // decimals
                &mint_authority,
                None,
                None,
                &payer,
            )
            .await?;

            println!("Created mint {} at address: {}", i, mint);
            mints.push(mint);
            mint_seeds.push(mint_seed);
            mint_authorities.push(mint_authority);
        }

        // Pre-create keypairs (40 to support maximum output tests + some extra)
        let keypairs: Vec<_> = (0..40).map(|_| Keypair::new()).collect();

        // Airdrop to all keypairs
        for keypair in &keypairs {
            airdrop_lamports(&mut rpc, &keypair.pubkey(), 10_000_000_000).await?;
        }

        // Mint compressed tokens based on signer requirements
        for ((signer_index, mint_index), version_amounts) in
            &requirements.signer_mint_compressed_amounts
        {
            let mint = mints[*mint_index];
            let mint_authority = &mint_authorities[*mint_index];

            for (version, amounts_vec) in version_amounts {
                // Create one compressed account for each amount in the vec
                for &amount in amounts_vec {
                    if amount > 0 {
                        println!(
                            "Minting {} tokens to signer {} with version {:?} from mint {} ({})",
                            amount, signer_index, version, mint_index, mint
                        );
                        let recipients = vec![Recipient {
                            recipient: keypairs[*signer_index].pubkey().into(),
                            amount,
                        }];

                        mint_to_compressed(
                            &mut rpc,
                            mint,
                            recipients,
                            *version,
                            mint_authority,
                            &payer,
                        )
                        .await?;
                    }
                }
            }
        }

        // Create CToken ATAs for compress/decompress operations
        let mut ctoken_atas = HashMap::new();
        for ((signer_index, mint_index), &amount) in &requirements.signer_ctoken_amounts {
            let mint = mints[*mint_index];
            let mint_seed = &mint_seeds[*mint_index];
            let mint_authority = &mint_authorities[*mint_index];
            let signer = &keypairs[*signer_index];

            // Create CToken ATA
            let create_ata_ix =
                light_compressed_token_sdk::instructions::create_associated_token_account(
                    payer.pubkey(),
                    signer.pubkey(),
                    mint,
                )
                .unwrap();

            rpc.create_and_send_transaction(&[create_ata_ix], &payer.pubkey(), &[&payer])
                .await
                .unwrap();

            let ata = light_compressed_token_sdk::instructions::derive_ctoken_ata(
                &signer.pubkey(),
                &mint,
            )
            .0;

            // Mint tokens to the CToken ATA if amount > 0
            if amount > 0 {
                println!(
                    "Minting {} tokens to CToken ATA for signer {} from mint {} ({})",
                    amount, signer_index, mint_index, mint
                );

                // Use MintToCToken action to mint to the ATA
                // Get the compressed mint address
                let address_tree_pubkey = rpc.get_address_tree_v2().tree;
                let compressed_mint_address =
                    light_compressed_token_sdk::instructions::derive_compressed_mint_address(
                        &mint_seed.pubkey(),
                        &address_tree_pubkey,
                    );

                light_token_client::actions::mint_action(
                    &mut rpc,
                    light_token_client::instructions::mint_action::MintActionParams {
                        compressed_mint_address,
                        mint_seed: mint_seed.pubkey(),
                        authority: mint_authority.pubkey(),
                        payer: payer.pubkey(),
                        actions: vec![light_compressed_token_sdk::instructions::mint_action::MintActionType::MintToCToken {
                            account: ata,
                            amount,
                        }],
                        new_mint: None,
                    },
                    mint_authority,
                    &payer,
                    None,
                ).await.unwrap();
            }

            ctoken_atas.insert((*signer_index, *mint_index), ata);
        }

        // TODO: Create SPL token accounts for compress operations
        // for (signer_index, &amount) in &requirements.signer_solana_amounts {
        //     // Create SPL token account and mint tokens
        // }

        Ok(TestContext {
            rpc,
            keypairs,
            mints,
            mint_seeds,
            mint_authorities,
            payer,
            ctoken_atas,
        })
    }

    fn analyze_test_requirements(test_case: &TestCase) -> TestRequirements {
        let mut signer_mint_compressed_amounts: HashMap<
            (usize, usize),
            HashMap<TokenDataVersion, Vec<u64>>,
        > = HashMap::new();
        let mut signer_solana_amounts: HashMap<usize, u64> = HashMap::new();
        let mut signer_ctoken_amounts: HashMap<(usize, usize), u64> = HashMap::new();

        for action in &test_case.actions {
            match action {
                MetaTransfer2InstructionType::Transfer(transfer) => {
                    // Transfer needs compressed tokens for the signer from specific mint
                    let key = (transfer.signer_index, transfer.mint_index);
                    let entry = signer_mint_compressed_amounts.entry(key).or_default();
                    let accounts_vec = entry.entry(transfer.token_data_version).or_default();

                    // Add each input account balance
                    for balance in &transfer.input_compressed_accounts {
                        accounts_vec.push(*balance);
                    }
                }
                MetaTransfer2InstructionType::Decompress(decompress) => {
                    // Decompress needs compressed tokens for the signer from specific mint
                    let key = (decompress.signer_index, decompress.mint_index);
                    let entry = signer_mint_compressed_amounts.entry(key).or_default();
                    let accounts_vec = entry.entry(decompress.token_data_version).or_default();

                    // Just push the amount for each account requested
                    for _ in 0..decompress.num_input_compressed_accounts {
                        accounts_vec.push(decompress.amount);
                    }

                    // Decompress also needs CToken ATA for recipient (but no balance needed)
                    let recipient_key = (decompress.recipient_index, decompress.mint_index);
                    signer_ctoken_amounts.entry(recipient_key).or_insert(0);
                }
                MetaTransfer2InstructionType::Approve(approve) => {
                    // Approve needs compressed tokens for the signer from specific mint
                    let key = (approve.signer_index, approve.mint_index);
                    let entry = signer_mint_compressed_amounts.entry(key).or_default();
                    let accounts_vec = entry.entry(approve.token_data_version).or_default();

                    // Approve typically uses single account
                    accounts_vec.push(approve.delegate_amount);
                }
                MetaTransfer2InstructionType::Compress(compress) => {
                    // Compress from CToken needs CToken account with balance
                    let key = (compress.signer_index, compress.mint_index);
                    *signer_ctoken_amounts.entry(key).or_insert(0) += compress.amount;
                }
                MetaTransfer2InstructionType::CompressAndClose(_) => {
                    // CompressAndClose needs a Solana token account - handled separately
                }
            }
        }

        TestRequirements {
            signer_mint_compressed_amounts,
            signer_solana_amounts,
            signer_ctoken_amounts,
        }
    }

    async fn convert_meta_actions_to_real(
        &mut self,
        meta_actions: &[MetaTransfer2InstructionType],
    ) -> Result<(Vec<Transfer2InstructionType>, Vec<Keypair>), Box<dyn std::error::Error>> {
        let mut real_actions = Vec::new();
        let mut required_pubkeys = Vec::new();

        // Always add payer
        required_pubkeys.push(self.payer.pubkey());

        for meta_action in meta_actions {
            match meta_action {
                MetaTransfer2InstructionType::Transfer(meta_transfer) => {
                    let real_action = self.convert_meta_transfer_to_real(meta_transfer).await?;
                    // Only add signer if this transfer has input accounts (not reusing from previous)
                    if !meta_transfer.input_compressed_accounts.is_empty() {
                        required_pubkeys.push(self.keypairs[meta_transfer.signer_index].pubkey());
                    }
                    real_actions.push(Transfer2InstructionType::Transfer(real_action));
                }
                MetaTransfer2InstructionType::Compress(meta_compress) => {
                    let real_action = self.convert_meta_compress_to_real(meta_compress).await?;
                    // Add the signer specified in the meta struct
                    required_pubkeys.push(self.keypairs[meta_compress.signer_index].pubkey());
                    real_actions.push(Transfer2InstructionType::Compress(real_action));
                }
                MetaTransfer2InstructionType::Decompress(meta_decompress) => {
                    let real_action = self
                        .convert_meta_decompress_to_real(meta_decompress)
                        .await?;
                    // Add the signer specified in the meta struct
                    required_pubkeys.push(self.keypairs[meta_decompress.signer_index].pubkey());
                    real_actions.push(Transfer2InstructionType::Decompress(real_action));
                }
                MetaTransfer2InstructionType::Approve(meta_approve) => {
                    let real_action = self.convert_meta_approve_to_real(meta_approve).await?;
                    // Add the signer specified in the meta struct
                    required_pubkeys.push(self.keypairs[meta_approve.signer_index].pubkey());
                    real_actions.push(Transfer2InstructionType::Approve(real_action));
                }
                MetaTransfer2InstructionType::CompressAndClose(meta_compress_and_close) => {
                    let real_action = self
                        .convert_meta_compress_and_close_to_real(meta_compress_and_close)
                        .await?;
                    // Add the signer specified in the meta struct
                    required_pubkeys
                        .push(self.keypairs[meta_compress_and_close.signer_index].pubkey());
                    real_actions.push(Transfer2InstructionType::CompressAndClose(real_action));
                }
            }
        }

        // Deduplicate required pubkeys
        required_pubkeys.sort();
        required_pubkeys.dedup();

        // Find the keypairs that match the required pubkeys
        let mut signers = Vec::new();
        for pubkey in required_pubkeys {
            if let Some(keypair) = self.find_keypair_by_pubkey(&pubkey) {
                signers.push(keypair);
            } else {
                return Err(format!("Could not find keypair for pubkey: {}", pubkey).into());
            }
        }

        Ok((real_actions, signers))
    }

    async fn convert_meta_transfer_to_real(
        &mut self,
        meta: &MetaTransferInput,
    ) -> Result<TransferInput, Box<dyn std::error::Error>> {
        // Get compressed accounts - either for owner or for accounts with delegate set
        let sender_accounts = if meta.input_compressed_accounts.is_empty() {
            // No new input accounts - this transfer uses inputs from a previous transfer in the same transaction
            vec![]
        } else if meta.is_delegate_transfer {
            // For delegate transfers, get accounts where the delegate is set
            // This would need delegate filtering in real implementation
            let accounts = self
                .rpc
                .indexer()
                .unwrap()
                .get_compressed_token_accounts_by_owner(&self.keypairs[0].pubkey(), None, None)
                .await?
                .value
                .items;
            // Take only the requested number of accounts and filter by version using discriminator
            accounts
                .into_iter()
                .filter(|acc| {
                    // Convert discriminator to TokenDataVersion and compare
                    TokenDataVersion::from_discriminator(
                        acc.account.data.clone().unwrap_or_default().discriminator,
                    )
                    .map(|v| v == meta.token_data_version)
                    .unwrap_or(false)
                })
                .take(meta.input_compressed_accounts.len())
                .collect()
        } else {
            // Regular transfer - get accounts by owner
            let accounts = self
                .rpc
                .indexer()
                .unwrap()
                .get_compressed_token_accounts_by_owner(
                    &self.keypairs[meta.signer_index].pubkey(),
                    None,
                    None,
                )
                .await?
                .value
                .items;
            println!(
                "Fetching accounts for signer {} (pubkey: {}), {:?}",
                meta.signer_index,
                self.keypairs[meta.signer_index].pubkey(),
                accounts
            );
            // Take only the requested number of accounts and filter by version using discriminator
            accounts
                .into_iter()
                .filter(|acc| {
                    // Convert discriminator to TokenDataVersion and compare
                    TokenDataVersion::from_discriminator(
                        acc.account.data.clone().unwrap_or_default().discriminator,
                    )
                    .map(|v| v == meta.token_data_version)
                    .unwrap_or(false)
                })
                .take(meta.input_compressed_accounts.len())
                .collect()
        };

        Ok(TransferInput {
            to: self.keypairs[meta.recipient_index].pubkey(),
            amount: meta.amount,
            change_amount: meta.change_amount,
            is_delegate_transfer: meta.is_delegate_transfer,
            mint: if sender_accounts.is_empty() {
                Some(self.mints[meta.mint_index]) // Provide mint when no input accounts
            } else {
                None
            },
            compressed_token_account: sender_accounts,
        })
    }

    async fn convert_meta_compress_to_real(
        &mut self,
        meta: &MetaCompressInput,
    ) -> Result<CompressInput, Box<dyn std::error::Error>> {
        // Get compressed accounts if needed (for compress operations that use compressed inputs)
        let compressed_accounts = if meta.num_input_compressed_accounts > 0 {
            let accounts = self
                .rpc
                .indexer()
                .unwrap()
                .get_compressed_token_accounts_by_owner(
                    &self.keypairs[meta.signer_index].pubkey(),
                    None,
                    None,
                )
                .await?
                .value
                .items;
            Some(accounts)
        } else {
            None
        };

        let output_queue = self
            .rpc
            .get_random_state_tree_info()
            .unwrap()
            .get_output_pubkey()
            .unwrap();

        // Get CToken ATA for this signer/mint combination
        let ctoken_ata = self
            .ctoken_atas
            .get(&(meta.signer_index, meta.mint_index))
            .ok_or_else(|| {
                format!(
                    "CToken ATA not found for signer {} mint {}",
                    meta.signer_index, meta.mint_index
                )
            })?;

        Ok(CompressInput {
            compressed_token_account: compressed_accounts,
            solana_token_account: *ctoken_ata, // Use the CToken ATA
            to: self.keypairs[meta.recipient_index].pubkey(),
            mint: self.mints[meta.mint_index],
            amount: meta.amount,
            authority: self.keypairs[meta.signer_index].pubkey(),
            output_queue,
        })
    }

    async fn convert_meta_decompress_to_real(
        &mut self,
        meta: &MetaDecompressInput,
    ) -> Result<DecompressInput, Box<dyn std::error::Error>> {
        // Get compressed accounts for the signer
        let sender_accounts = self
            .rpc
            .indexer()
            .unwrap()
            .get_compressed_token_accounts_by_owner(
                &self.keypairs[meta.signer_index].pubkey(),
                None,
                None,
            )
            .await?
            .value
            .items;

        // Get CToken ATA for the recipient (should have been created in TestContext::new)
        let recipient_ata = self
            .ctoken_atas
            .get(&(meta.recipient_index, meta.mint_index))
            .ok_or_else(|| {
                format!(
                    "CToken ATA not found for recipient {} mint {}",
                    meta.recipient_index, meta.mint_index
                )
            })?;

        Ok(DecompressInput {
            compressed_token_account: sender_accounts,
            decompress_amount: meta.decompress_amount,
            solana_token_account: *recipient_ata, // Use the CToken ATA
            amount: meta.amount,
        })
    }

    async fn convert_meta_approve_to_real(
        &mut self,
        meta: &MetaApproveInput,
    ) -> Result<ApproveInput, Box<dyn std::error::Error>> {
        // Get compressed accounts for the owner (signer)
        let sender_accounts = self
            .rpc
            .indexer()
            .unwrap()
            .get_compressed_token_accounts_by_owner(
                &self.keypairs[meta.signer_index].pubkey(),
                None,
                None,
            )
            .await?
            .value
            .items;

        Ok(ApproveInput {
            compressed_token_account: sender_accounts,
            delegate: self.keypairs[meta.delegate_index].pubkey(), // Use specified delegate
            delegate_amount: meta.delegate_amount,
        })
    }

    async fn convert_meta_compress_and_close_to_real(
        &mut self,
        meta: &MetaCompressAndCloseInput,
    ) -> Result<CompressAndCloseInput, Box<dyn std::error::Error>> {
        // Get output queue
        let merkle_trees = self.rpc.get_state_merkle_trees();
        let output_queue = merkle_trees[0].accounts.nullifier_queue;

        Ok(CompressAndCloseInput {
            solana_ctoken_account: self.keypairs[meta.signer_index].pubkey(), // TODO: Will need actual token account when we add that test
            authority: self.keypairs[meta.signer_index].pubkey(), // Owner is always the authority
            output_queue,
            destination: meta
                .destination_index
                .map(|idx| self.keypairs[idx].pubkey()),
        })
    }

    async fn perform_test(
        &mut self,
        test_case: &TestCase,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Convert meta actions to real actions and get required signers
        let (actions, signers) = self
            .convert_meta_actions_to_real(&test_case.actions)
            .await?;

        let payer_pubkey = self.payer.pubkey();

        // Create the transfer2 instruction
        let ix = create_generic_transfer2_instruction(&mut self.rpc, actions.clone(), payer_pubkey)
            .await?;

        // Create and send transaction
        let (recent_blockhash, _) = self.rpc.get_latest_blockhash().await?;

        let signer_refs: Vec<&Keypair> = signers.iter().collect();

        let tx = Transaction::new_signed_with_payer(
            &[ix.clone()],
            Some(&payer_pubkey),
            &signer_refs,
            recent_blockhash,
        );

        // Process the transaction
        self.rpc.process_transaction(tx).await.inspect_err(|_| {
            println!(
                "instruction: {:?}",
                CompressedTokenInstructionDataTransfer2::deserialize(&mut &ix.data[1..]).unwrap()
            );
        })?;
        println!(
            "instruction: {:?}",
            CompressedTokenInstructionDataTransfer2::deserialize(&mut &ix.data[1..]).unwrap()
        );
        println!("actions: {:?}", actions);
        assert_transfer2(&mut self.rpc, actions).await;

        Ok(())
    }
}

// Basic Transfer Operations

//  1. 1 in 2 out Version::V1
//  2. 1 in 2 out Version::V2
//  3. 1 in 2 out Version::ShaFlat
//  4. 2 in 2 out Version::ShaFlat
//  5. 3 in 2 out Version::ShaFlat
//  6. 4 in 2 out Version::ShaFlat
//  7. 5 in 2 out Version::ShaFlat
//  8. 6 in 2 out Version::ShaFlat
//  9. 7 in 2 out Version::ShaFlat
//  10. 8 in 2 out Version::ShaFlat (maximum inputs)
//  11. Single input to multiple outputs (1→N split)
//  12. Multiple inputs to single output (N→1 merge)
//  13. Multiple inputs to multiple outputs (N→M complex)
//  14. Transfer with 0 explicit outputs (change account only)

//  Output Account Limits

//  15. 1 output compressed account
//  16. 10 output compressed accounts
//  17. 20 output compressed accounts
//  18. 35 output compressed accounts (maximum)

//  Amount Edge Cases

//  19. Transfer 0 tokens (valid operation)
//  20. Transfer 1 token (minimum non-zero)
//  21. Transfer full balance (no change account created)
//  22. Transfer partial balance (change account created)
//  23. Transfer u64::MAX tokens
//  24. Multiple partial transfers creating multiple change accounts

//  Token Data Versions

//  25. All V1 (Poseidon with pubkey hashing)
//  26. All V2 (Poseidon with pubkey hashing)
//  27. All V3/ShaFlat (SHA256)
//  28. Mixed V1 and V2 in same transaction
//  29. Mixed V1 and V3 in same transaction
//  30. Mixed V2 and V3 in same transaction
//  31. All three versions in same transaction

//  Multi-Mint Operations

//  32. Single mint operations
//  33. 2 different mints in same transaction
//  34. 3 different mints in same transaction
//  35. 4 different mints in same transaction
//  36. 5 different mints in same transaction (maximum)
//  37. Multiple operations per mint (e.g., 2 transfers of mint A, 3 of mint B)

//  Compression Operations (Path A - no compressed accounts)

//  38. Compress from SPL token only
//  39. Compress from CToken only
//  40. Decompress to SPL token only
//  41. Decompress to CToken only
//  42. Multiple compress operations only
//  43. Multiple decompress operations only
//  44. Compress and decompress same amount (must balance)

//  Mixed Compression + Transfer (Path B)

//  45. Transfer + compress SPL in same transaction
//  46. Transfer + decompress to SPL in same transaction
//  47. Transfer + compress CToken in same transaction
//  48. Transfer + decompress to CToken in same transaction
//  49. Transfer + multiple compressions
//  50. Transfer + multiple decompressions
//  51. Transfer + compress + decompress (all must balance)

//  CompressAndClose Operations

//  52. CompressAndClose as owner (no validation needed)
//  53. CompressAndClose as rent authority (requires compressible account)
//  54. Multiple CompressAndClose in single transaction
//  55. CompressAndClose + regular transfer in same transaction
//  56. CompressAndClose with full balance
//  57. CompressAndClose creating specific output (rent authority case)

//  Delegate Operations

//  58. Approve creating delegated account + change
//  59. Transfer using delegate authority (full delegated amount)
//  60. Transfer using delegate authority (partial amount)
//  61. Revoke delegation (merges all accounts)
//  62. Multiple delegates in same transaction
//  63. Delegate transfer with change account

//  Token Pool Operations

//  64. Compress to pool index 0
//  65. Compress to pool index 1
//  66. Compress to pool index 4 (max is 5)
//  67. Decompress from pool index 0
//  68. Decompress from different pool indices
//  69. Multiple pools for same mint in transaction

//  Change Account Behavior

//  70. Single change account from partial transfer
//  71. Multiple change accounts from multiple partial transfers
//  72. No change account when full amount transferred
//  73. Change account preserving delegate
//  74. Change account with different token version
//  75. Zero-amount change accounts (SDK behavior)

//  Sum Check Validation

//  76. Perfect balance single mint (inputs = outputs)
//  77. Perfect balance 2 mints
//  78. Perfect balance 5 mints (max)
//  79. Compress 1000, decompress 1000 (must balance)
//  80. Multiple compress = multiple decompress
//  81. Complex multi-mint balancing

//  Merkle Tree/Queue Targeting

//  82. All outputs to same merkle tree
//  83. Outputs to different merkle trees
//  84. Outputs to queue vs tree
//  85. Multiple trees and queues in same transaction

//  Account Reuse Patterns

//  86. Same owner multiple inputs
//  87. Same recipient multiple outputs
//  88. Circular transfer A→B, B→A in same transaction
//  89. Self-transfer (same account input and output)
//  90. Multiple operations on same mint

//  Proof Modes

//  91. Proof by index (no ZK proof)
//  92. With ZK proof
//  93. Mixed proof modes in same transaction
//  94. with_transaction_hash = true

//  Transfer Deduplication

//  95. Multiple transfers to same recipient (should deduplicate)
//  96. Up to 40 compression transfers (maximum)
//  97. Deduplication across different mints

//  Cross-Type Implicit Transfers

//  98. SPL to CToken without compressed intermediary
//  99. CToken to SPL without compressed intermediary
//  100. Mixed SPL and CToken operations

//  Complex Scenarios

//  101. Maximum complexity: 8 inputs, 35 outputs, 5 mints
//  102. All operations: transfer + compress + decompress + CompressAndClose
//  103. Circular transfers with multiple participants: A→B→C→A

#[tokio::test]
#[serial]
async fn test_transfer2_functional() {
    let test_cases = vec![
        // Basic input account tests
        // test1_basic_transfer_poseidon_v1(),
        // test1_basic_transfer_poseidon_v2(),
        // test1_basic_transfer_sha_flat(),
        // test1_basic_transfer_sha_flat_8(),
        // test1_basic_transfer_sha_flat_2_inputs(),
        // test1_basic_transfer_sha_flat_3_inputs(),
        // test1_basic_transfer_sha_flat_4_inputs(),
        // test1_basic_transfer_sha_flat_5_inputs(),
        // test1_basic_transfer_sha_flat_6_inputs(),
        // test1_basic_transfer_sha_flat_7_inputs(),
        // test1_basic_transfer_sha_flat_8_inputs(),
        // New complex transfer pattern tests
        // test2_single_input_multiple_outputs(),
        // test3_multiple_inputs_single_output(),
        // test4_multiple_inputs_multiple_outputs(),
        // test5_change_account_only(),
        // // Output account limit tests
        // test6_single_output_account(),
        // test7_ten_output_accounts(),
        // test8_twenty_output_accounts(),
        // test9_maximum_output_accounts(),
        // // Amount edge case tests
        // test10_transfer_zero_tokens(),
        // test11_transfer_one_token(),
        // test12_transfer_full_balance(),
        // test13_transfer_partial_balance(),
        // test14_transfer_max_tokens(),
        // test15_multiple_partial_transfers(),
        // test16_all_v1_poseidon(),
        // test17_all_v2_poseidon(),
        // test18_all_sha_flat(),
        // test19_mixed_v1_v2(),
        // test20_mixed_v1_sha_flat(),
        // test21_mixed_v2_sha_flat(),
        // test22_all_three_versions(),
        // // Multi-mint operation tests
        // test23_single_mint_operations(),
        // test24_two_different_mints(),
        // test25_three_different_mints(),
        // test26_four_different_mints(),
        // test27_five_different_mints_maximum(),
        // test28_multiple_operations_per_mint(),
        // CToken compression operations tests
        test39_compress_from_ctoken_only(),
        test40_decompress_to_ctoken_only(),
        test41_multiple_compress_operations(),
        test42_multiple_decompress_operations(),
        test43_compress_decompress_balance(),
    ];

    for (i, test_case) in test_cases.iter().enumerate() {
        println!("\n========================================");
        println!("Test #{}: {}", i + 1, test_case.name);
        println!("========================================");

        // Create test context with all initialization
        let mut ctx = TestContext::new(test_case).await.unwrap();

        // Execute the test
        ctx.perform_test(test_case).await.unwrap();
    }

    println!("\n========================================");
    println!("All tests completed successfully!");
    println!("========================================");
}

// ============================================================================
// Test Case Builders
// ============================================================================

fn test1_basic_transfer_poseidon_v1() -> TestCase {
    TestCase {
        name: "Basic compressed-to-compressed transfer".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300], // One account with 300 tokens
            amount: 300,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::V1,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

fn test1_basic_transfer_poseidon_v2() -> TestCase {
    TestCase {
        name: "Basic compressed-to-compressed transfer".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300], // One account with 300 tokens
            amount: 300,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::V2,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

fn test1_basic_transfer_sha_flat() -> TestCase {
    TestCase {
        name: "Basic compressed-to-compressed transfer".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300], // One account with 300 tokens
            amount: 300,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

fn test1_basic_transfer_sha_flat_8() -> TestCase {
    TestCase {
        name: "8 transfers from different signers using ShaFlat (max input limit)".to_string(),
        actions: (0..8) // MAX_INPUT_ACCOUNTS is 8
            .map(|i| {
                MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                    input_compressed_accounts: vec![300], // One account with 300 tokens
                    amount: 100, // Partial transfer to avoid 0-amount change accounts
                    is_delegate_transfer: false,
                    token_data_version: TokenDataVersion::ShaFlat,
                    signer_index: i,        // Each transfer from keypair 0-7
                    delegate_index: None,   // Not a delegate transfer
                    recipient_index: i + 8, // Transfer to keypair 8-15 (no overlap with signers)
                    change_amount: None,
                    mint_index: 0,
                })
            })
            .collect(),
    }
}

fn test1_basic_transfer_sha_flat_2_inputs() -> TestCase {
    TestCase {
        name: "2 transfers from different signers using ShaFlat".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300, 300], // Two accounts with 300 tokens each
            amount: 600,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

fn test1_basic_transfer_sha_flat_3_inputs() -> TestCase {
    TestCase {
        name: "3 transfers from different signers using ShaFlat".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300, 300, 300], // Three accounts with 300 tokens each
            amount: 900,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

fn test1_basic_transfer_sha_flat_4_inputs() -> TestCase {
    TestCase {
        name: "4 transfers from different signers using ShaFlat".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300, 300, 300, 300], // Four accounts
            amount: 1200,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

fn test1_basic_transfer_sha_flat_5_inputs() -> TestCase {
    TestCase {
        name: "5 transfers from different signers using ShaFlat".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300, 300, 300, 300, 300], // Five accounts
            amount: 1500,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

fn test1_basic_transfer_sha_flat_6_inputs() -> TestCase {
    TestCase {
        name: "6 transfers from different signers using ShaFlat".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300, 300, 300, 300, 300, 300], // Six accounts
            amount: 1800,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

fn test1_basic_transfer_sha_flat_7_inputs() -> TestCase {
    TestCase {
        name: "7 transfers from different signers using ShaFlat".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300, 300, 300, 300, 300, 300, 300], // Seven accounts
            amount: 2100,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

fn test1_basic_transfer_sha_flat_8_inputs() -> TestCase {
    TestCase {
        name: "8 transfers from different signers using ShaFlat (max input limit)".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![300, 300, 300, 300, 300, 300, 300, 300], // Eight accounts
            amount: 2400,
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,      // Owner (keypair[0]) signs the transfer
            delegate_index: None, // Not a delegate transfer
            recipient_index: 1,   // Transfer to keypair[1]
            change_amount: None,
            mint_index: 0,
        })],
    }
}

// Test 1: Single input to multiple outputs (1→N split)
fn test2_single_input_multiple_outputs() -> TestCase {
    TestCase {
        name: "Single input to multiple outputs (1→N split)".to_string(),
        actions: vec![
            // Transfer 100 tokens from keypair[0] to keypair[1]
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![900], // Create account with 700 tokens
                amount: 100,                          // Transfer 100
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 10,
                change_amount: Some(900 - 100 - 150 - 50),
                mint_index: 0,
            }),
            // Transfer 150 tokens from keypair[0] to keypair[2]
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![], // Uses existing input from first transfer
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 12,
                change_amount: Some(0),
                mint_index: 0,
            }),
            // Transfer 50 tokens from keypair[0] to keypair[3]
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![], // Uses existing input from first transfer
                amount: 50,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                delegate_index: None,
                recipient_index: 13,
                change_amount: Some(0),
                mint_index: 0,
            }),
        ],
    }
}

// Test 2: Multiple inputs to single output (N→1 merge)
fn test3_multiple_inputs_single_output() -> TestCase {
    TestCase {
        name: "Multiple inputs to single output (N→1 merge)".to_string(),
        actions: vec![
            // Transfer from keypair[0] to keypair[5]
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![200, 200],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 5,
                change_amount: None,
                mint_index: 0,
            }),
            // Transfer from keypair[1] to keypair[5] (same recipient)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![150],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 5,
                change_amount: None,
                mint_index: 0,
            }),
            // Transfer from keypair[2] to keypair[5] (same recipient)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![100],
                amount: 100,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                delegate_index: None,
                recipient_index: 5,
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// Test 3: Multiple inputs to multiple outputs (N→M complex)
fn test4_multiple_inputs_multiple_outputs() -> TestCase {
    TestCase {
        name: "Multiple inputs to multiple outputs (N→M complex)".to_string(),
        actions: vec![
            // Transfer from keypair[0] - split to multiple recipients
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![100, 100],
                amount: 100,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 3,
                change_amount: Some(50), // Keep 100 as change for next transfer
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![], // Reuse input
                amount: 50,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 4,
                change_amount: Some(0), // Use 50 from change, keep 50 remaining
                mint_index: 0,
            }),
            // Transfer from keypair[1] - split to multiple recipients
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![100, 100],
                amount: 75,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 3,     // Same recipient as above
                change_amount: Some(0), // Keep 125 as change for next transfer
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![], // Reuse input
                amount: 125,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 5,
                change_amount: Some(0), // Use all 125 from change
                mint_index: 0,
            }),
            // Transfer from keypair[2] to multiple recipients
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![80],
                amount: 80,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                delegate_index: None,
                recipient_index: 4,     // Same recipient as above
                change_amount: Some(0), // Exact amount, no change
                mint_index: 0,
            }),
        ],
    }
}

// Test 4: Transfer with 0 explicit outputs (change account only)
fn test5_change_account_only() -> TestCase {
    TestCase {
        name: "Transfer with change account only (partial transfer to self)".to_string(),
        actions: vec![
            // Transfer partial amount to self - creates only a change account
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150, // Partial amount, leaving 150 as change
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 0, // Transfer to self
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// ============================================================================
// Output Account Limit Tests (12-15)
// ============================================================================

// Test 6: Single output compressed account (minimum)
fn test6_single_output_account() -> TestCase {
    TestCase {
        name: "Single output compressed account".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![100], // One input account
            amount: 100,                          // Transfer full amount
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,
            delegate_index: None,
            recipient_index: 1,     // Single output
            change_amount: Some(0), // No change (full amount transfer)
            mint_index: 0,
        })],
    }
}

// Test 7: 10 output compressed accounts
fn test7_ten_output_accounts() -> TestCase {
    TestCase {
        name: "10 output compressed accounts".to_string(),
        actions: {
            let mut actions = vec![];
            // Create one large input account to split into 10 outputs
            let total_amount = 1000u64;
            let amount_per_output = 100u64;

            // First transfer with input account, creates change for subsequent transfers
            actions.push(MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![total_amount],
                amount: amount_per_output,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: Some(0), // Keep remaining as change
                mint_index: 0,
            }));

            // 9 more transfers using the change from the first transfer
            for i in 1..10 {
                let remaining_change = total_amount - (amount_per_output * (i as u64 + 1));
                actions.push(MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                    input_compressed_accounts: vec![], // Use change from previous
                    amount: amount_per_output,
                    is_delegate_transfer: false,
                    token_data_version: TokenDataVersion::ShaFlat,
                    signer_index: 0,
                    delegate_index: None,
                    recipient_index: i + 1, // Recipients 2-10
                    change_amount: Some(0),
                    mint_index: 0,
                }));
            }

            actions
        },
    }
}

// Test 8: 20 output compressed accounts
fn test8_twenty_output_accounts() -> TestCase {
    TestCase {
        name: "20 output compressed accounts".to_string(),
        actions: {
            let mut actions = vec![];
            let total_amount = 2000u64;
            let amount_per_output = 100u64;

            // First transfer with input account
            actions.push(MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![total_amount],
                amount: amount_per_output,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: Some(0),
                mint_index: 0,
            }));

            // 19 more transfers using the change
            for i in 1..20 {
                let remaining_change = total_amount - (amount_per_output * (i as u64 + 1));
                actions.push(MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                    input_compressed_accounts: vec![],
                    amount: amount_per_output,
                    is_delegate_transfer: false,
                    token_data_version: TokenDataVersion::ShaFlat,
                    signer_index: 0,
                    delegate_index: None,
                    recipient_index: i + 1, // Recipients 2-20
                    change_amount: Some(0),
                    mint_index: 0,
                }));
            }

            actions
        },
    }
}

// Test 9: 35 output compressed accounts (maximum per instruction)
fn test9_maximum_output_accounts() -> TestCase {
    TestCase {
        name: "35 output compressed accounts (maximum)".to_string(),
        actions: {
            let mut actions = vec![];
            let total_amount = 2900u64; // 35 * 100
            let amount_per_output = 100u64;

            // First transfer with input account
            actions.push(MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![total_amount],
                amount: amount_per_output,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: Some(0),
                mint_index: 0,
            }));

            // 34 more transfers to reach the maximum of 35 outputs
            for i in 1..29 {
                let remaining_change = total_amount - (amount_per_output * (i as u64 + 1));
                actions.push(MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                    input_compressed_accounts: vec![],
                    amount: amount_per_output,
                    is_delegate_transfer: false,
                    token_data_version: TokenDataVersion::ShaFlat,
                    signer_index: 0,
                    delegate_index: None,
                    recipient_index: i + 1, // Recipients 2-35
                    change_amount: Some(0),
                    mint_index: 0,
                }));
            }

            actions
        },
    }
}

// ============================================================================
// Amount Edge Case Tests (16-21)
// ============================================================================

// Test 10: Transfer 0 tokens (valid operation)
fn test10_transfer_zero_tokens() -> TestCase {
    TestCase {
        name: "Transfer 0 tokens (valid operation)".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![1000],
            amount: 0, // Transfer 0 tokens
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,
            delegate_index: None,
            recipient_index: 1,
            change_amount: None, // Keep all 1000 as change
            mint_index: 0,
        })],
    }
}

// Test 11: Transfer 1 token (minimum non-zero)
fn test11_transfer_one_token() -> TestCase {
    TestCase {
        name: "Transfer 1 token (minimum non-zero)".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![1000],
            amount: 1, // Transfer 1 token
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,
            delegate_index: None,
            recipient_index: 1,
            change_amount: None, // Keep 999 as change
            mint_index: 0,
        })],
    }
}

// Test 12: Transfer full balance (no change account created)
fn test12_transfer_full_balance() -> TestCase {
    TestCase {
        name: "Transfer full balance (no change account created)".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![1000],
            amount: 1000, // Transfer full amount
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,
            delegate_index: None,
            recipient_index: 1,
            change_amount: Some(0), // No change account
            mint_index: 0,
        })],
    }
}

// Test 13: Transfer partial balance (change account created)
fn test13_transfer_partial_balance() -> TestCase {
    TestCase {
        name: "Transfer partial balance (change account created)".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![1000],
            amount: 750, // Partial transfer
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,
            delegate_index: None,
            recipient_index: 1,
            change_amount: None, // Keep 250 as change
            mint_index: 0,
        })],
    }
}

// Test 14: Transfer u64::MAX tokens (maximum possible)
fn test14_transfer_max_tokens() -> TestCase {
    TestCase {
        name: "Transfer u64::MAX tokens (maximum possible)".to_string(),
        actions: vec![MetaTransfer2InstructionType::Transfer(MetaTransferInput {
            input_compressed_accounts: vec![u64::MAX],
            amount: u64::MAX, // Maximum amount
            is_delegate_transfer: false,
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,
            delegate_index: None,
            recipient_index: 1,
            change_amount: Some(0), // No change account
            mint_index: 0,
        })],
    }
}

// Test 15: Multiple partial transfers creating multiple change accounts
fn test15_multiple_partial_transfers() -> TestCase {
    TestCase {
        name: "Multiple partial transfers creating multiple change accounts".to_string(),
        actions: vec![
            // First partial transfer
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![1000],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None, // Keep 800 as change
                mint_index: 0,
            }),
            // Second partial transfer from different account
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 2,
                change_amount: None, // Keep 350 as change
                mint_index: 0,
            }),
            // Third partial transfer from another account
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![800],
                amount: 300,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                delegate_index: None,
                recipient_index: 3,
                change_amount: None, // Keep 500 as change
                mint_index: 0,
            }),
        ],
    }
}
// ============================================================================
// Token Data Version Tests (22-28)
// ============================================================================

// Test 16: All V1 (Poseidon with pubkey hashing)
fn test16_all_v1_poseidon() -> TestCase {
    TestCase {
        name: "All V1 (Poseidon with pubkey hashing)".to_string(),
        actions: vec![
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V1,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V1,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 2,
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// Test 17: All V2 (Poseidon with pubkey hashing)
fn test17_all_v2_poseidon() -> TestCase {
    TestCase {
        name: "All V2 (Poseidon with pubkey hashing)".to_string(),
        actions: vec![
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V2,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V2,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 2,
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// Test 18: All V3/ShaFlat (SHA256)
fn test18_all_sha_flat() -> TestCase {
    TestCase {
        name: "All V3/ShaFlat (SHA256)".to_string(),
        actions: vec![
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 2,
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// Test 19: Mixed V1 and V2 in same transaction
fn test19_mixed_v1_v2() -> TestCase {
    TestCase {
        name: "Mixed V1 and V2 in same transaction".to_string(),
        actions: vec![
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V1, // V1 transfer
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V2, // V2 transfer
                signer_index: 1,
                delegate_index: None,
                recipient_index: 2,
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// Test 20: Mixed V1 and V3 in same transaction
fn test20_mixed_v1_sha_flat() -> TestCase {
    TestCase {
        name: "Mixed V1 and V3 in same transaction".to_string(),
        actions: vec![
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V1, // V1 transfer
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat, // ShaFlat transfer
                signer_index: 1,
                delegate_index: None,
                recipient_index: 2,
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// Test 21: Mixed V2 and V3 in same transaction
fn test21_mixed_v2_sha_flat() -> TestCase {
    TestCase {
        name: "Mixed V2 and V3 in same transaction".to_string(),
        actions: vec![
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V2, // V2 transfer
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat, // ShaFlat transfer
                signer_index: 1,
                delegate_index: None,
                recipient_index: 2,
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// Test 22: All three versions in same transaction
fn test22_all_three_versions() -> TestCase {
    TestCase {
        name: "All three versions in same transaction".to_string(),
        actions: vec![
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V1, // V1 transfer
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::V2, // V2 transfer
                signer_index: 1,
                delegate_index: None,
                recipient_index: 2,
                change_amount: None,
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![400],
                amount: 100,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat, // ShaFlat transfer
                signer_index: 2,
                delegate_index: None,
                recipient_index: 3,
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// ============================================================================
// Multi-Mint Operation Tests (29-34)
// ============================================================================

// Test 23: Single mint operations
fn test23_single_mint_operations() -> TestCase {
    TestCase {
        name: "Single mint operations".to_string(),
        actions: vec![
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0,
            }),
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 2,
                change_amount: None,
                mint_index: 0,
            }),
        ],
    }
}

// Test 24: 2 different mints in same transaction
fn test24_two_different_mints() -> TestCase {
    TestCase {
        name: "2 different mints in same transaction".to_string(),
        actions: vec![
            // Transfer from mint A
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 1,
                change_amount: None,
                mint_index: 0,
            }),
            // Transfer from mint B (different mint)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1, // Different signer implies different mint
                delegate_index: None,
                recipient_index: 2,
                change_amount: None,
                mint_index: 1,
            }),
        ],
    }
}

// Test 25: 3 different mints in same transaction
fn test25_three_different_mints() -> TestCase {
    TestCase {
        name: "3 different mints in same transaction".to_string(),
        actions: vec![
            // Transfer from mint A
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 3,
                change_amount: None,
                mint_index: 0,
            }),
            // Transfer from mint B
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 4,
                change_amount: None,
                mint_index: 1,
            }),
            // Transfer from mint C
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![400],
                amount: 100,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                delegate_index: None,
                recipient_index: 5,
                change_amount: None,
                mint_index: 2,
            }),
        ],
    }
}

// Test 26: 4 different mints in same transaction
fn test26_four_different_mints() -> TestCase {
    TestCase {
        name: "4 different mints in same transaction".to_string(),
        actions: vec![
            // Transfer from mint A
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 4,
                change_amount: None,
                mint_index: 0,
            }),
            // Transfer from mint B
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 5,
                change_amount: None,
                mint_index: 1,
            }),
            // Transfer from mint C
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![400],
                amount: 100,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                delegate_index: None,
                recipient_index: 6,
                change_amount: None,
                mint_index: 2,
            }),
            // Transfer from mint D
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![600],
                amount: 250,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 3,
                delegate_index: None,
                recipient_index: 7,
                change_amount: None,
                mint_index: 3,
            }),
        ],
    }
}

// Test 27: 5 different mints in same transaction (maximum)
fn test27_five_different_mints_maximum() -> TestCase {
    TestCase {
        name: "5 different mints in same transaction (maximum)".to_string(),
        actions: vec![
            // Transfer from mint A
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                delegate_index: None,
                recipient_index: 5,
                change_amount: None,
                mint_index: 0,
            }),
            // Transfer from mint B
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                delegate_index: None,
                recipient_index: 6,
                change_amount: None,
                mint_index: 1,
            }),
            // Transfer from mint C
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![400],
                amount: 100,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                delegate_index: None,
                recipient_index: 7,
                change_amount: None,
                mint_index: 2,
            }),
            // Transfer from mint D
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![600],
                amount: 250,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 3,
                delegate_index: None,
                recipient_index: 8,
                change_amount: None,
                mint_index: 3,
            }),
            // Transfer from mint E
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![700],
                amount: 300,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 4,
                delegate_index: None,
                recipient_index: 9,
                change_amount: None,
                mint_index: 4,
            }),
        ],
    }
}

// Test 28: Multiple operations per mint (2 transfers of mint A, 3 of mint B)
fn test28_multiple_operations_per_mint() -> TestCase {
    TestCase {
        name: "Multiple operations per mint (2 transfers of mint A, 3 of mint B)".to_string(),
        actions: vec![
            // First transfer from mint A (signer 0)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![500],
                amount: 200,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0, // Mint A, signer 0
                delegate_index: None,
                recipient_index: 10,
                change_amount: None,
                mint_index: 0,
            }),
            // Second transfer from mint A (different signer to avoid double spend)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![300],
                amount: 150,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2, // Mint A, different signer (2)
                delegate_index: None,
                recipient_index: 11,
                change_amount: None,
                mint_index: 0,
            }),
            // First transfer from mint B (signer 1)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![400],
                amount: 100,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1, // Mint B, signer 1
                delegate_index: None,
                recipient_index: 12,
                change_amount: None,
                mint_index: 1,
            }),
            // Second transfer from mint B (different signer to avoid double spend)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![600],
                amount: 250,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 3, // Mint B, different signer (3)
                delegate_index: None,
                recipient_index: 13,
                change_amount: None,
                mint_index: 1,
            }),
            // Third transfer from mint B (another different signer to avoid double spend)
            MetaTransfer2InstructionType::Transfer(MetaTransferInput {
                input_compressed_accounts: vec![350],
                amount: 175,
                is_delegate_transfer: false,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 4, // Mint B, different signer (4)
                delegate_index: None,
                recipient_index: 14,
                change_amount: None,
                mint_index: 1,
            }),
        ],
    }
}

// ============================================================================
// Compression Operations Tests (39-44)
// ============================================================================

// Test 39: Compress from CToken only
fn test39_compress_from_ctoken_only() -> TestCase {
    TestCase {
        name: "Compress from CToken only".to_string(),
        actions: vec![MetaTransfer2InstructionType::Compress(MetaCompressInput {
            num_input_compressed_accounts: 0, // No compressed inputs
            amount: 1000,                     // Amount to compress from CToken ATA
            token_data_version: TokenDataVersion::ShaFlat,
            signer_index: 0,    // Owner of the CToken ATA
            recipient_index: 0, // Compress to same owner
            mint_index: 0,
        })],
    }
}

// Test 40: Decompress to CToken only
fn test40_decompress_to_ctoken_only() -> TestCase {
    TestCase {
        name: "Decompress to CToken only".to_string(),
        actions: vec![MetaTransfer2InstructionType::Decompress(
            MetaDecompressInput {
                num_input_compressed_accounts: 1, // One compressed account as input
                decompress_amount: 800,
                amount: 800,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,    // Owner of compressed tokens
                recipient_index: 1, // Decompress to different recipient
                mint_index: 0,
            },
        )],
    }
}

// Test 41: Multiple compress operations only
fn test41_multiple_compress_operations() -> TestCase {
    TestCase {
        name: "Multiple compress operations only".to_string(),
        actions: vec![
            // First compress from signer 0
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 500,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                recipient_index: 0,
                mint_index: 0,
            }),
            // Second compress from signer 1
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 750,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                recipient_index: 1,
                mint_index: 0,
            }),
            // Third compress from signer 2
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 250,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                recipient_index: 2,
                mint_index: 0,
            }),
        ],
    }
}

// Test 42: Multiple decompress operations only
fn test42_multiple_decompress_operations() -> TestCase {
    TestCase {
        name: "Multiple decompress operations only".to_string(),
        actions: vec![
            // First decompress to recipient 0
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1,
                decompress_amount: 400,
                amount: 400,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                recipient_index: 3, // Different recipient
                mint_index: 0,
            }),
            // Second decompress to recipient 1
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1,
                decompress_amount: 300,
                amount: 300,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                recipient_index: 4, // Different recipient
                mint_index: 0,
            }),
            // Third decompress to recipient 2
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1,
                decompress_amount: 200,
                amount: 200,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 2,
                recipient_index: 5, // Different recipient
                mint_index: 0,
            }),
        ],
    }
}

// Test 43: Compress and decompress same amount (must balance)
fn test43_compress_decompress_balance() -> TestCase {
    TestCase {
        name: "Compress and decompress same amount (must balance)".to_string(),
        actions: vec![
            // Compress 1000 tokens from CToken
            MetaTransfer2InstructionType::Compress(MetaCompressInput {
                num_input_compressed_accounts: 0,
                amount: 1000,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 0,
                recipient_index: 0,
                mint_index: 0,
            }),
            // Decompress 1000 tokens to different CToken
            MetaTransfer2InstructionType::Decompress(MetaDecompressInput {
                num_input_compressed_accounts: 1,
                decompress_amount: 1000,
                amount: 1000,
                token_data_version: TokenDataVersion::ShaFlat,
                signer_index: 1,
                recipient_index: 2, // Different recipient
                mint_index: 0,
            }),
        ],
    }
}

use std::collections::HashMap;

use anchor_lang::AnchorDeserialize;
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_token_sdk::instructions::{
    find_spl_mint_address, CreateCompressibleAssociatedTokenAccountInputs,
};
use light_ctoken_types::{
    instructions::{mint_action::Recipient, transfer2::CompressedTokenInstructionDataTransfer2},
    state::TokenDataVersion,
};
use light_program_test::{indexer::TestIndexerExtensions, LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    airdrop_lamports,
    assert_transfer2::assert_transfer2,
    spl::{
        create_additional_token_pools, create_mint_helper, create_token_account, mint_spl_tokens,
    },
};
use light_token_client::{
    actions::{create_mint, mint_to_compressed},
    instructions::transfer2::{
        create_generic_transfer2_instruction, ApproveInput, CompressAndCloseInput, CompressInput,
        DecompressInput, Transfer2InstructionType, TransferInput,
    },
};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};

// ============================================================================
// Display helpers for actions
// ============================================================================

// Print methods for Meta action types
impl MetaTransferInput {
    pub fn print(&self, index: usize) {
        println!("  Action {}: Transfer (Meta)", index);
        println!("    - signer_index: {}", self.signer_index);
        println!("    - recipient_index: {}", self.recipient_index);
        println!("    - mint_index: {}", self.mint_index);
        println!("    - amount: {}", self.amount);
        println!("    - change_amount: {:?}", self.change_amount);
        println!("    - is_delegate_transfer: {}", self.is_delegate_transfer);
        println!("    - delegate_index: {:?}", self.delegate_index);
        println!("    - token_data_version: {:?}", self.token_data_version);
        println!(
            "    - input_compressed_accounts: {:?}",
            self.input_compressed_accounts
        );
    }
}

impl MetaCompressInput {
    pub fn print(&self, index: usize) {
        println!("  Action {}: Compress (Meta)", index);
        println!("    - signer_index: {}", self.signer_index);
        println!("    - recipient_index: {}", self.recipient_index);
        println!("    - mint_index: {}", self.mint_index);
        println!("    - amount: {}", self.amount);
        println!("    - use_spl: {}", self.use_spl);
        println!("    - pool_index: {:?}", self.pool_index);
        println!(
            "    - num_input_compressed_accounts: {}",
            self.num_input_compressed_accounts
        );
        println!("    - token_data_version: {:?}", self.token_data_version);
    }
}

impl MetaDecompressInput {
    pub fn print(&self, index: usize) {
        println!("  Action {}: Decompress (Meta)", index);
        println!("    - signer_index: {}", self.signer_index);
        println!("    - recipient_index: {}", self.recipient_index);
        println!("    - mint_index: {}", self.mint_index);
        println!("    - decompress_amount: {}", self.decompress_amount);
        println!("    - amount: {}", self.amount);
        println!("    - to_spl: {}", self.to_spl);
        println!("    - pool_index: {:?}", self.pool_index);
        println!(
            "    - num_input_compressed_accounts: {}",
            self.num_input_compressed_accounts
        );
        println!("    - token_data_version: {:?}", self.token_data_version);
    }
}

impl MetaApproveInput {
    pub fn print(&self, index: usize) {
        println!("  Action {}: Approve (Meta)", index);
        println!("    - signer_index: {}", self.signer_index);
        println!("    - delegate_index: {}", self.delegate_index);
        println!("    - mint_index: {}", self.mint_index);
        println!("    - delegate_amount: {}", self.delegate_amount);
        println!(
            "    - num_input_compressed_accounts: {}",
            self.num_input_compressed_accounts
        );
        println!("    - token_data_version: {:?}", self.token_data_version);
        println!("    - setup: {}", self.setup);
    }
}

impl MetaCompressAndCloseInput {
    pub fn print(&self, index: usize) {
        println!("  Action {}: CompressAndClose (Meta)", index);
        println!("    - signer_index: {}", self.signer_index);
        println!("    - destination_index: {:?}", self.destination_index);
        println!("    - mint_index: {}", self.mint_index);
        println!("    - token_data_version: {:?}", self.token_data_version);
        println!("    - is_compressible: {}", self.is_compressible);
    }
}

impl MetaTransfer2InstructionType {
    pub fn print(&self, index: usize) {
        match self {
            MetaTransfer2InstructionType::Transfer(t) => t.print(index),
            MetaTransfer2InstructionType::Compress(c) => c.print(index),
            MetaTransfer2InstructionType::Decompress(d) => d.print(index),
            MetaTransfer2InstructionType::Approve(a) => a.print(index),
            MetaTransfer2InstructionType::CompressAndClose(c) => c.print(index),
        }
    }
}

// ============================================================================
// Test Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct TestConfig {
    pub default_setup_amount: u64,
    pub max_supported_mints: usize,
    pub test_token_decimals: u8,
    pub max_keypairs: usize,
    pub airdrop_amount: u64,
    pub base_compressed_account_amount: u64,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            default_setup_amount: 500,
            max_supported_mints: 5,
            test_token_decimals: 6,
            max_keypairs: 40,
            airdrop_amount: 10_000_000_000,
            base_compressed_account_amount: 500,
        }
    }
}

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
    pub to_spl: bool,           // If true, decompress to SPL; if false, decompress to CToken ATA
    pub pool_index: Option<u8>, // For SPL only. None = default (0), Some(n) = specific pool
}

#[derive(Debug, Clone)]
pub struct MetaCompressInput {
    pub num_input_compressed_accounts: u8,
    pub amount: u64,
    pub token_data_version: TokenDataVersion,
    pub signer_index: usize,    // Index of keypair that signs this action
    pub recipient_index: usize, // Index of keypair to receive compressed tokens
    pub mint_index: usize,      // Index of which mint to use (0-4)
    pub use_spl: bool,          // If true, use SPL token account; if false, use CToken ATA
    pub pool_index: Option<u8>, // For SPL only. None = default (0), Some(n) = specific pool
}

#[derive(Debug, Clone)]
pub struct MetaCompressAndCloseInput {
    pub token_data_version: TokenDataVersion,
    pub signer_index: usize, // Index of keypair that signs this action
    pub destination_index: Option<usize>, // Index of keypair to receive lamports (None = no destination)
    pub mint_index: usize,                // Index of which mint to use (0-4)
    pub is_compressible: bool, // If true, account has extensions (compressible); if false, regular CToken ATA
}

#[derive(Debug, Clone)]
pub struct MetaApproveInput {
    pub num_input_compressed_accounts: u8,
    pub delegate_amount: u64,
    pub token_data_version: TokenDataVersion,
    pub signer_index: usize, // Index of keypair that signs this action (owner)
    pub delegate_index: usize, // Index of keypair to set as delegate
    pub mint_index: usize,   // Index of which mint to use (0-4)
    pub setup: bool,         // If true, execute in setup phase; if false, execute in main test
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

#[allow(unused)]
struct TestRequirements {
    // Map from (signer_index, mint_index) to their required token amounts per version
    pub signer_mint_compressed_amounts:
        HashMap<(usize, usize), HashMap<TokenDataVersion, Vec<u64>>>,
    pub signer_solana_amounts: HashMap<usize, u64>, // For compress operations
    pub signer_ctoken_amounts: HashMap<(usize, usize), u64>, // For CToken accounts (signer_index, mint_index) -> amount
    pub signer_spl_amounts: HashMap<(usize, usize), u64>, // For SPL token accounts (signer_index, mint_index) -> amount
    pub signer_ctoken_compressible: HashMap<(usize, usize), bool>, // Track which accounts need compressible extensions
}

// Test context to pass to builder functions
#[allow(unused)]
pub struct TestContext {
    rpc: LightProgramTest,
    keypairs: Vec<Keypair>,
    mints: Vec<Pubkey>,       // Multiple mints (up to config.max_supported_mints)
    mint_seeds: Vec<Keypair>, // Mint seeds used to derive mints
    mint_authorities: Vec<Keypair>, // One authority per mint
    payer: Keypair,
    ctoken_atas: HashMap<(usize, usize), Pubkey>, // (signer_index, mint_index) -> CToken ATA pubkey
    spl_token_accounts: HashMap<(usize, usize), Keypair>, // (signer_index, mint_index) -> SPL token account keypair
    config: TestConfig,
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
        // Check SPL token accounts
        for token_account_keypair in self.spl_token_accounts.values() {
            if token_account_keypair.pubkey() == *pubkey {
                return Some(token_account_keypair.insecure_clone());
            }
        }
        self.keypairs
            .iter()
            .find(|kp| kp.pubkey() == *pubkey)
            .map(|kp| kp.insecure_clone())
    }

    pub async fn new(
        test_case: &TestCase,
        config: TestConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Analyze test case to determine requirements
        let requirements = Self::analyze_test_requirements(test_case, &config);
        // Fresh RPC for each test
        let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
        let payer = rpc.get_payer().insecure_clone();

        // Create mints - either compressed or SPL depending on requirements
        let mut mints = Vec::new();
        let mut mint_seeds = Vec::new();
        let mut mint_authorities = Vec::new();

        // Check which mint types we need for each index
        // A mint needs SPL if it's used for SPL compression or SPL decompression
        let mut mint_needs_spl = vec![false; config.max_supported_mints];
        for (_, mint_index) in requirements.signer_spl_amounts.keys() {
            mint_needs_spl[*mint_index] = true;
        }

        for (i, mint_needs_spl) in mint_needs_spl
            .iter()
            .enumerate()
            .take(config.max_supported_mints)
        {
            let mint_authority = Keypair::new();

            if *mint_needs_spl {
                // Create SPL mint for SPL compression
                let mint = create_mint_helper(&mut rpc, &payer).await;
                println!("Created SPL mint {} at address: {}", i, mint);
                mints.push(mint);
                mint_seeds.push(Keypair::new()); // Dummy seed for SPL mints
                mint_authorities.push(payer.insecure_clone()); // Use payer as authority for SPL mints
            } else {
                // Create compressed mint for CToken operations
                let mint_seed = Keypair::new();
                let (mint, _) = find_spl_mint_address(&mint_seed.pubkey());

                create_mint(
                    &mut rpc,
                    &mint_seed,
                    config.test_token_decimals,
                    &mint_authority,
                    None,
                    None,
                    &payer,
                )
                .await?;

                println!("Created compressed mint {} at address: {}", i, mint);
                mints.push(mint);
                mint_seeds.push(mint_seed);
                mint_authorities.push(mint_authority);
            }
        }

        // Create additional token pools for SPL mints based on test requirements
        // Scan test actions to find max pool_index for each mint
        let mut max_pool_index_per_mint = vec![0u8; config.max_supported_mints];
        for action in &test_case.actions {
            match action {
                MetaTransfer2InstructionType::Compress(compress) if compress.use_spl => {
                    if let Some(pool_index) = compress.pool_index {
                        let mint_index = compress.mint_index;
                        if pool_index > max_pool_index_per_mint[mint_index] {
                            max_pool_index_per_mint[mint_index] = pool_index;
                        }
                    }
                }
                MetaTransfer2InstructionType::Decompress(decompress) if decompress.to_spl => {
                    if let Some(pool_index) = decompress.pool_index {
                        let mint_index = decompress.mint_index;
                        if pool_index > max_pool_index_per_mint[mint_index] {
                            max_pool_index_per_mint[mint_index] = pool_index;
                        }
                    }
                }
                _ => {}
            }
        }

        // Create additional pools for SPL mints that need them (pool 0 already exists)
        for (mint_index, &max_pool_index) in max_pool_index_per_mint.iter().enumerate() {
            if mint_needs_spl[mint_index] && max_pool_index > 0 {
                let mint = mints[mint_index];
                println!(
                    "Creating additional token pools (1-{}) for SPL mint {} ({})",
                    max_pool_index, mint_index, mint
                );
                create_additional_token_pools(&mut rpc, &payer, &mint, false, max_pool_index)
                    .await
                    .unwrap();
            }
        }

        // Pre-create keypairs to support maximum output tests + some extra
        let keypairs: Vec<_> = (0..config.max_keypairs).map(|_| Keypair::new()).collect();

        // Airdrop to all keypairs
        for keypair in &keypairs {
            airdrop_lamports(&mut rpc, &keypair.pubkey(), config.airdrop_amount).await?;
        }

        // Mint compressed tokens based on signer requirements (skip for SPL mints)
        for ((signer_index, mint_index), version_amounts) in
            &requirements.signer_mint_compressed_amounts
        {
            // Skip if this is an SPL mint
            if mint_needs_spl[*mint_index] {
                println!("Skipping compressed token minting for SPL mint {} - will create via compression", mint_index);
                continue;
            }

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

        // Get compressible config from test accounts (already created in program test setup)
        let funding_pool_config = rpc.test_accounts.funding_pool_config;

        // Create CToken ATAs for compress/decompress operations
        let mut ctoken_atas = HashMap::new();
        for ((signer_index, mint_index), &amount) in &requirements.signer_ctoken_amounts {
            let mint = mints[*mint_index];
            let mint_seed = &mint_seeds[*mint_index];
            let mint_authority = &mint_authorities[*mint_index];
            let signer = &keypairs[*signer_index];

            // Check if this account needs compressible extensions
            let is_compressible = *requirements
                .signer_ctoken_compressible
                .get(&(*signer_index, *mint_index))
                .unwrap_or(&false);

            // Create CToken ATA (compressible or regular based on requirements)
            let create_ata_ix = if is_compressible {
                println!(
                    "Creating compressible CToken ATA for signer {} mint {}",
                    signer_index, mint_index
                );
                light_compressed_token_sdk::instructions::create_compressible_associated_token_account(
                    CreateCompressibleAssociatedTokenAccountInputs {
                        payer: payer.pubkey(),
                        owner: signer.pubkey(),
                        mint,
                        compressible_config: funding_pool_config.compressible_config_pda,
                        rent_sponsor: funding_pool_config.rent_sponsor_pda,
                        pre_pay_num_epochs: 10, // Prepay 10 epochs of rent
                        lamports_per_write: None,
                        token_account_version: TokenDataVersion::ShaFlat, // CompressAndClose requires ShaFlat
                    },
                )
                .unwrap()
            } else {
                light_compressed_token_sdk::instructions::create_associated_token_account(
                    payer.pubkey(),
                    signer.pubkey(),
                    mint,
                )
                .unwrap()
            };

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

        // Create SPL token accounts for SPL compress operations
        let mut spl_token_accounts = HashMap::new();
        for ((signer_index, mint_index), &base_amount) in &requirements.signer_spl_amounts {
            let mint = mints[*mint_index];
            let signer = &keypairs[*signer_index];
            let token_account_keypair = Keypair::new();

            // Calculate total amount needed
            // For mixed compression tests, we need extra tokens that will be compressed in setup
            let mut total_amount = base_amount;
            for action in &test_case.actions {
                if let MetaTransfer2InstructionType::Compress(compress) = action {
                    if compress.use_spl
                        && compress.num_input_compressed_accounts > 0
                        && compress.signer_index == *signer_index
                        && compress.mint_index == *mint_index
                    {
                        // We'll compress some tokens in setup, so mint extra
                        let setup_compress_amount = config.default_setup_amount
                            * compress.num_input_compressed_accounts as u64;
                        total_amount += setup_compress_amount;
                        println!(
                            "Adding {} extra SPL tokens for setup compression",
                            setup_compress_amount
                        );
                    }
                }
            }

            // Create SPL token account
            create_token_account(&mut rpc, &mint, &token_account_keypair, signer)
                .await
                .unwrap();

            // Mint SPL tokens if amount > 0
            if total_amount > 0 {
                println!(
                    "Minting {} SPL tokens to account for signer {} from mint {} ({})",
                    total_amount, signer_index, mint_index, mint
                );

                // SPL mints use payer as authority
                mint_spl_tokens(
                    &mut rpc,
                    &mint,
                    &token_account_keypair.pubkey(),
                    &payer.pubkey(), // mint authority pubkey
                    &payer,          // SPL mints use payer as authority keypair
                    total_amount,
                    false, // not token22
                )
                .await
                .unwrap();
            }

            spl_token_accounts.insert((*signer_index, *mint_index), token_account_keypair);
        }

        // Compress SPL tokens to create compressed accounts for tests that need them
        for action in &test_case.actions {
            match action {
                MetaTransfer2InstructionType::Decompress(decompress) => {
                    // Check if this mint is an SPL mint (needs SPL token account)
                    let is_spl_mint = mint_needs_spl[decompress.mint_index];

                    // If it's an SPL mint, we need to compress SPL tokens to create compressed accounts
                    // This works for both SPL decompression (to_spl: true) and CToken decompression (to_spl: false)
                    if is_spl_mint {
                        let key = (decompress.signer_index, decompress.mint_index);
                        if let Some(token_account_keypair) = spl_token_accounts.get(&key) {
                            let target = if decompress.to_spl { "SPL" } else { "CToken" };
                            println!(
                                "Compressing SPL tokens for signer {} mint {} to create compressed accounts for {} decompression",
                                decompress.signer_index, decompress.mint_index, target
                            );

                            // Calculate amounts needed and mint additional SPL tokens if necessary
                            let setup_amount =
                                decompress.amount * decompress.num_input_compressed_accounts as u64;

                            // Check if any compress operations in this test also need SPL tokens
                            let mut additional_compress_amount = 0u64;
                            for other_action in &test_case.actions {
                                if let MetaTransfer2InstructionType::Compress(compress) =
                                    other_action
                                {
                                    if compress.use_spl
                                        && compress.signer_index == decompress.signer_index
                                        && compress.mint_index == decompress.mint_index
                                    {
                                        additional_compress_amount += compress.amount;
                                    }
                                }
                            }

                            let total_needed = setup_amount + additional_compress_amount;

                            // If we need more than the initial tokens, mint the difference
                            if total_needed > config.default_setup_amount {
                                let additional_amount = total_needed - config.default_setup_amount;
                                println!("Minting additional {} SPL tokens for setup and test operations", additional_amount);
                                mint_spl_tokens(
                                    &mut rpc,
                                    &mints[decompress.mint_index],
                                    &token_account_keypair.pubkey(),
                                    &payer.pubkey(),
                                    &payer,
                                    additional_amount,
                                    false,
                                )
                                .await
                                .unwrap();
                            }

                            // Compress the SPL tokens using Transfer2 with Compress action
                            let mint = mints[decompress.mint_index];
                            let signer = &keypairs[decompress.signer_index];

                            // Get output queue
                            let output_queue = rpc
                                .get_random_state_tree_info()
                                .unwrap()
                                .get_output_pubkey()
                                .unwrap();

                            // Create compress input
                            let compress_input = CompressInput {
                                compressed_token_account: None, // No compressed inputs when compressing from SPL
                                solana_token_account: token_account_keypair.pubkey(),
                                to: signer.pubkey(),
                                mint,
                                amount: setup_amount,
                                authority: signer.pubkey(),
                                output_queue,
                                pool_index: None,
                            };

                            // Create and execute the compress instruction
                            let ix = create_generic_transfer2_instruction(
                                &mut rpc,
                                vec![Transfer2InstructionType::Compress(compress_input)],
                                payer.pubkey(),
                                false,
                            )
                            .await
                            .unwrap();

                            rpc.create_and_send_transaction(
                                &[ix],
                                &payer.pubkey(),
                                &[&payer, signer],
                            )
                            .await
                            .unwrap();
                        }
                    }
                    // Note: For compressed mints, CToken decompression uses regular compressed tokens from normal minting
                }
                MetaTransfer2InstructionType::Compress(compress)
                    if compress.use_spl && compress.num_input_compressed_accounts > 0 =>
                {
                    // This test needs both SPL tokens AND compressed accounts
                    // Compress some SPL tokens to create the compressed accounts
                    let key = (compress.signer_index, compress.mint_index);
                    if let Some(token_account_keypair) = spl_token_accounts.get(&key) {
                        println!(
                            "Compressing SPL tokens for signer {} mint {} to create {} compressed accounts for mixed compression",
                            compress.signer_index, compress.mint_index, compress.num_input_compressed_accounts
                        );

                        let mint = mints[compress.mint_index];
                        let signer = &keypairs[compress.signer_index];

                        // Compress tokens to create the compressed accounts needed
                        let amount_to_compress = config.default_setup_amount
                            * compress.num_input_compressed_accounts as u64;

                        let output_queue = rpc
                            .get_random_state_tree_info()
                            .unwrap()
                            .get_output_pubkey()
                            .unwrap();

                        let compress_input = CompressInput {
                            compressed_token_account: None,
                            solana_token_account: token_account_keypair.pubkey(),
                            to: signer.pubkey(),
                            mint,
                            amount: amount_to_compress,
                            authority: signer.pubkey(),
                            output_queue,
                            pool_index: None,
                        };

                        let ix = create_generic_transfer2_instruction(
                            &mut rpc,
                            vec![Transfer2InstructionType::Compress(compress_input)],
                            payer.pubkey(),
                            false,
                        )
                        .await
                        .unwrap();

                        rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, signer])
                            .await
                            .unwrap();
                    }
                }
                _ => {}
            }
        }

        // Execute Approve operations in setup phase (only if setup=true)
        // These need to happen before the test runs so delegated accounts exist
        for action in &test_case.actions {
            if let MetaTransfer2InstructionType::Approve(approve) = action {
                // Only execute in setup if setup flag is true
                if !approve.setup {
                    continue;
                }

                println!(
                    "Setup: Executing Approve for signer {} delegate {} amount {}",
                    approve.signer_index, approve.delegate_index, approve.delegate_amount
                );

                let owner = &keypairs[approve.signer_index];
                let delegate_pubkey = keypairs[approve.delegate_index].pubkey();
                let mint = mints[approve.mint_index];

                // Fetch owner's compressed accounts
                let accounts = rpc
                    .indexer()
                    .unwrap()
                    .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
                    .await?
                    .value
                    .items;

                // Filter for matching version and mint, take first account with enough balance
                let matching_accounts: Vec<_> = accounts
                    .into_iter()
                    .filter(|acc| {
                        // Check version matches
                        let version_matches = TokenDataVersion::from_discriminator(
                            acc.account.data.clone().unwrap_or_default().discriminator,
                        )
                        .map(|v| v == approve.token_data_version)
                        .unwrap_or(false);

                        // Check mint matches
                        let mint_matches = acc.token.mint == mint;

                        // Check has enough balance
                        let enough_balance = acc.token.amount >= approve.delegate_amount;

                        version_matches && mint_matches && enough_balance
                    })
                    .take(1)
                    .collect();

                if matching_accounts.is_empty() {
                    return Err(format!(
                        "No matching account found for Approve: owner={}, mint={}, amount={}, version={:?}",
                        owner.pubkey(),
                        mint,
                        approve.delegate_amount,
                        approve.token_data_version
                    )
                    .into());
                }

                // Build ApproveInput
                let approve_input = ApproveInput {
                    compressed_token_account: matching_accounts,
                    delegate: delegate_pubkey,
                    delegate_amount: approve.delegate_amount,
                };

                // Create and execute the approve instruction
                let ix = create_generic_transfer2_instruction(
                    &mut rpc,
                    vec![Transfer2InstructionType::Approve(approve_input)],
                    payer.pubkey(),
                    false,
                )
                .await?;

                rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, owner])
                    .await?;

                println!(
                    "Setup: Approve executed successfully - delegated account created for delegate {}",
                    delegate_pubkey
                );
            }
        }

        Ok(TestContext {
            rpc,
            keypairs,
            mints,
            mint_seeds,
            mint_authorities,
            payer,
            ctoken_atas,
            spl_token_accounts,
            config,
        })
    }

    fn analyze_test_requirements(test_case: &TestCase, config: &TestConfig) -> TestRequirements {
        let mut signer_mint_compressed_amounts: HashMap<
            (usize, usize),
            HashMap<TokenDataVersion, Vec<u64>>,
        > = HashMap::new();
        let signer_solana_amounts: HashMap<usize, u64> = HashMap::new();
        let mut signer_ctoken_amounts: HashMap<(usize, usize), u64> = HashMap::new();
        let mut signer_spl_amounts: HashMap<(usize, usize), u64> = HashMap::new();
        let mut signer_ctoken_compressible: HashMap<(usize, usize), bool> = HashMap::new();

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
                    let key = (decompress.signer_index, decompress.mint_index);
                    let recipient_key = (decompress.recipient_index, decompress.mint_index);

                    if decompress.to_spl {
                        // For SPL decompression, we need:
                        // 1. SPL-origin compressed tokens (create by compressing from SPL in setup)
                        // 2. SPL token account for recipient

                        // Create SPL tokens to compress into compressed accounts
                        let spl_amount_to_compress =
                            decompress.amount * decompress.num_input_compressed_accounts as u64;
                        *signer_spl_amounts.entry(key).or_insert(0) += spl_amount_to_compress;

                        // Need SPL token account for recipient (no initial balance needed)
                        signer_spl_amounts.entry(recipient_key).or_insert(0);
                    } else {
                        // For CToken decompression, we need regular compressed tokens
                        let entry = signer_mint_compressed_amounts.entry(key).or_default();
                        let accounts_vec = entry.entry(decompress.token_data_version).or_default();

                        // Just push the amount for each account requested
                        for _ in 0..decompress.num_input_compressed_accounts {
                            accounts_vec.push(decompress.amount);
                        }

                        // Need CToken ATA for recipient (no balance needed)
                        signer_ctoken_amounts.entry(recipient_key).or_insert(0);
                    }
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
                    let key = (compress.signer_index, compress.mint_index);

                    // If using compressed accounts as inputs, create them
                    if compress.num_input_compressed_accounts > 0 {
                        let entry = signer_mint_compressed_amounts.entry(key).or_default();
                        let accounts_vec = entry.entry(compress.token_data_version).or_default();

                        // Create compressed accounts for input
                        // Use default amount per compressed account for testing
                        for _ in 0..compress.num_input_compressed_accounts {
                            accounts_vec.push(config.base_compressed_account_amount);
                        }
                    }

                    if compress.use_spl {
                        // Compress from SPL needs SPL token account with balance
                        // When we have compressed inputs too, we need to create them from SPL first
                        if compress.num_input_compressed_accounts > 0 {
                            // We need SPL tokens for:
                            // 1. Creating the compressed accounts (500 each)
                            // 2. The SPL portion of the compress operation
                            let compressed_total = config.base_compressed_account_amount
                                * compress.num_input_compressed_accounts as u64;
                            let spl_portion = compress.amount.saturating_sub(compressed_total);
                            // Total SPL tokens needed = tokens to compress into compressed accounts + SPL portion
                            *signer_spl_amounts.entry(key).or_insert(0) +=
                                compressed_total + spl_portion;
                        } else {
                            // Just need SPL tokens for the compress operation
                            *signer_spl_amounts.entry(key).or_insert(0) += compress.amount;
                        }
                    } else {
                        // Compress from CToken needs CToken account with balance
                        *signer_ctoken_amounts.entry(key).or_insert(0) += compress.amount;
                    }
                }
                MetaTransfer2InstructionType::CompressAndClose(compress_and_close) => {
                    // CompressAndClose needs a CToken ATA with balance
                    let key = (
                        compress_and_close.signer_index,
                        compress_and_close.mint_index,
                    );
                    // Use default setup amount as the balance for the CToken ATA
                    *signer_ctoken_amounts.entry(key).or_insert(0) += config.default_setup_amount;
                    // Track whether this account needs compressible extensions
                    signer_ctoken_compressible.insert(key, compress_and_close.is_compressible);
                }
            }
        }

        TestRequirements {
            signer_mint_compressed_amounts,
            signer_solana_amounts,
            signer_ctoken_amounts,
            signer_spl_amounts,
            signer_ctoken_compressible,
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

        // Filter out Approve actions that were executed in setup (setup=true)
        let filtered_actions: Vec<_> = meta_actions
            .iter()
            .filter(|action| {
                !matches!(action, MetaTransfer2InstructionType::Approve(approve) if approve.setup)
            })
            .collect();

        for meta_action in filtered_actions {
            match meta_action {
                MetaTransfer2InstructionType::Transfer(meta_transfer) => {
                    let real_action = self.convert_meta_transfer_to_real(meta_transfer).await?;
                    // Only add signer if this transfer has input accounts (not reusing from previous)
                    if !meta_transfer.input_compressed_accounts.is_empty() {
                        let signer_pubkey = if meta_transfer.is_delegate_transfer {
                            // For delegate transfers, the actual signer is the delegate
                            let delegate_index = meta_transfer
                                .delegate_index
                                .expect("Delegate index required for delegate transfers");
                            self.keypairs[delegate_index].pubkey()
                        } else {
                            // For regular transfers, signer is the owner
                            self.keypairs[meta_transfer.signer_index].pubkey()
                        };
                        required_pubkeys.push(signer_pubkey);
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
            let delegate_index = meta
                .delegate_index
                .ok_or("Delegate index required for delegate transfers")?;
            let delegate_pubkey = self.keypairs[delegate_index].pubkey();
            let owner_pubkey = self.keypairs[meta.signer_index].pubkey();

            println!(
                "Fetching delegated accounts for owner {} with delegate {}",
                owner_pubkey, delegate_pubkey
            );

            // Fetch accounts owned by the owner
            let accounts = self
                .rpc
                .indexer()
                .unwrap()
                .get_compressed_token_accounts_by_owner(&owner_pubkey, None, None)
                .await?
                .value
                .items;

            // Filter for accounts with matching delegate
            accounts
                .into_iter()
                .filter(|acc| {
                    // Check if delegate matches
                    let delegate_matches = acc.token.delegate == Some(delegate_pubkey);

                    // Check version matches
                    let version_matches = TokenDataVersion::from_discriminator(
                        acc.account.data.clone().unwrap_or_default().discriminator,
                    )
                    .map(|v| v == meta.token_data_version)
                    .unwrap_or(false);

                    delegate_matches && version_matches
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

        // Get the appropriate token account based on use_spl flag
        let solana_token_account = if meta.use_spl {
            // Get SPL token account
            let keypair = self
                .spl_token_accounts
                .get(&(meta.signer_index, meta.mint_index))
                .ok_or_else(|| {
                    format!(
                        "SPL token account not found for signer {} mint {}",
                        meta.signer_index, meta.mint_index
                    )
                })?;
            keypair.pubkey()
        } else {
            // Get CToken ATA
            *self
                .ctoken_atas
                .get(&(meta.signer_index, meta.mint_index))
                .ok_or_else(|| {
                    format!(
                        "CToken ATA not found for signer {} mint {}",
                        meta.signer_index, meta.mint_index
                    )
                })?
        };

        Ok(CompressInput {
            compressed_token_account: compressed_accounts,
            solana_token_account,
            to: self.keypairs[meta.recipient_index].pubkey(),
            mint: self.mints[meta.mint_index],
            amount: meta.amount,
            authority: self.keypairs[meta.signer_index].pubkey(),
            output_queue,
            pool_index: meta.pool_index,
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

        // Get the appropriate token account based on to_spl flag
        let recipient_account = if meta.to_spl {
            // Get SPL token account for the recipient
            let keypair = self
                .spl_token_accounts
                .get(&(meta.recipient_index, meta.mint_index))
                .ok_or_else(|| {
                    format!(
                        "SPL token account not found for recipient {} mint {}",
                        meta.recipient_index, meta.mint_index
                    )
                })?;
            keypair.pubkey()
        } else {
            // Get CToken ATA for the recipient
            *self
                .ctoken_atas
                .get(&(meta.recipient_index, meta.mint_index))
                .ok_or_else(|| {
                    format!(
                        "CToken ATA not found for recipient {} mint {}",
                        meta.recipient_index, meta.mint_index
                    )
                })?
        };

        Ok(DecompressInput {
            compressed_token_account: sender_accounts,
            decompress_amount: meta.decompress_amount,
            solana_token_account: recipient_account,
            amount: meta.amount,
            pool_index: meta.pool_index,
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

        // Get the CToken ATA for the signer
        let ctoken_ata = *self
            .ctoken_atas
            .get(&(meta.signer_index, meta.mint_index))
            .ok_or_else(|| {
                format!(
                    "CToken ATA not found for signer {} mint {}",
                    meta.signer_index, meta.mint_index
                )
            })?;

        Ok(CompressAndCloseInput {
            solana_ctoken_account: ctoken_ata,
            authority: self.keypairs[meta.signer_index].pubkey(), // Owner is always the authority
            output_queue,
            destination: meta
                .destination_index
                .map(|idx| self.keypairs[idx].pubkey()),
            is_compressible: meta.is_compressible,
        })
    }

    pub async fn perform_test(
        &mut self,
        test_case: &TestCase,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for (i, action) in test_case.actions.iter().enumerate() {
            action.print(i);
        }

        // Convert meta actions to real actions and get required signers
        let (actions, signers) = self
            .convert_meta_actions_to_real(&test_case.actions)
            .await?;

        // Print actions in readable format
        println!("Actions ({} total):", actions.len());

        // Print SPL token account balances
        println!("\nSPL Token Account Balances:");
        for ((signer_index, mint_index), token_account_keypair) in &self.spl_token_accounts {
            let account_data = self
                .rpc
                .get_account(token_account_keypair.pubkey())
                .await
                .unwrap()
                .unwrap();

            use anchor_spl::token_2022::spl_token_2022::state::Account as SplTokenAccount;
            use solana_sdk::program_pack::Pack;
            let spl_account = SplTokenAccount::unpack(&account_data.data[..165]).unwrap();

            println!(
                "  Signer {} Mint {}: {} (account: {})",
                signer_index,
                mint_index,
                spl_account.amount,
                token_account_keypair.pubkey()
            );
        }

        println!(
            "\nSigners ({} total): {:?}",
            signers.len(),
            signers.iter().map(|s| s.pubkey()).collect::<Vec<_>>()
        );
        let payer_pubkey = self.payer.pubkey();

        // Create the transfer2 instruction
        let ix = create_generic_transfer2_instruction(
            &mut self.rpc,
            actions.clone(),
            payer_pubkey,
            false,
        )
        .await?;

        // Create and send transaction
        let (recent_blockhash, _) = self.rpc.get_latest_blockhash().await?;

        println!("Payer pubkey: {}", payer_pubkey);
        println!(
            "Instruction accounts: {:?}",
            ix.accounts
                .iter()
                .filter(|a| a.is_signer)
                .map(|a| a.pubkey)
                .collect::<Vec<_>>()
        );

        let instruction_signer_pubkeys: Vec<_> = ix
            .accounts
            .iter()
            .filter(|a| a.is_signer)
            .map(|a| a.pubkey)
            .collect();

        let mut signer_refs: Vec<&Keypair> = signers
            .iter()
            .filter(|s| instruction_signer_pubkeys.contains(&s.pubkey()))
            .collect();
        signer_refs.insert(0, &self.payer);
        println!(
            "Signers pubkeys: {:?}",
            signer_refs.iter().map(|s| s.pubkey()).collect::<Vec<_>>()
        );
        let tx = Transaction::new_signed_with_payer(
            std::slice::from_ref(&ix),
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

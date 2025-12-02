//! Build load params - unified function for loading compressed accounts (PDAs + ctokens)

use light_client::indexer::{CompressedAccount, Indexer, IndexerError};
use light_client::rpc::Rpc;
use light_compressed_token_sdk::{
    ctoken::TransferSplToCtoken, token_pool::find_token_pool_pda_with_index,
};
use light_compressed_token_types::{
    SPL_ASSOCIATED_TOKEN_PROGRAM_ID, SPL_TOKEN_2022_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID,
};
use light_sdk::compressible::Pack;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use spl_pod::bytemuck::pod_from_bytes;
use spl_token_2022::pod::PodAccount;

use crate::{
    compressible_instruction::decompress_accounts_idempotent,
    get_compressible_account::{AccountInfoInterface, MerkleContext},
};

/// Input for build_load_params - a compressed account with its parsed data.
///
/// For programs using compressible tokens, `T` should be an enum like:
/// ```ignore
/// enum CompressedAccountVariant {
///     UserRecord(UserRecord),
///     GameSession(GameSession),
///     PackedCTokenData(PackedCTokenData<CTokenAccountVariant>),
///     // ... other variants
/// }
/// ```
///
/// This allows passing both PDAs and ctokens in a single call.
pub struct CompressibleAccountInput<T> {
    pub address: Pubkey,
    pub info: AccountInfoInterface,
    pub parsed: T,
}

impl<T> CompressibleAccountInput<T> {
    pub fn new(address: Pubkey, info: AccountInfoInterface, parsed: T) -> Self {
        Self {
            address,
            info,
            parsed,
        }
    }

    pub fn is_compressed(&self) -> bool {
        self.info.is_compressed
    }

    pub fn merkle_context(&self) -> Option<&MerkleContext> {
        self.info.merkle_context.as_ref()
    }
}

/// Get the derived SPL ATA address
fn get_spl_ata(owner: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    let associated_token_program = Pubkey::new_from_array(SPL_ASSOCIATED_TOKEN_PROGRAM_ID);
    Pubkey::find_program_address(
        &[owner.as_ref(), token_program.as_ref(), mint.as_ref()],
        &associated_token_program,
    )
    .0
}

/// Create a wrap instruction if there is a balance
async fn try_wrap_spl_balance<R: Rpc>(
    rpc: &mut R,
    owner: &Pubkey,
    mint: &Pubkey,
    payer: Pubkey,
    ctoken_ata: Pubkey,
    token_program: Pubkey,
) -> Result<Option<Instruction>, IndexerError> {
    let spl_ata = get_spl_ata(owner, mint, &token_program);

    let Some(account_info) = rpc
        .get_account(spl_ata)
        .await
        .map_err(|e| IndexerError::CustomError(e.to_string()))?
    else {
        return Ok(None);
    };

    let Ok(pod_account) = pod_from_bytes::<PodAccount>(&account_info.data) else {
        return Ok(None);
    };

    let balance: u64 = pod_account.amount.into();
    if balance == 0 {
        return Ok(None);
    }

    let (token_pool_pda, token_pool_pda_bump) = find_token_pool_pda_with_index(mint, 0);
    let wrap_ix = TransferSplToCtoken {
        amount: balance,
        token_pool_pda_bump,
        source_spl_token_account: spl_ata,
        destination_ctoken_account: ctoken_ata,
        authority: *owner,
        mint: *mint,
        payer,
        token_pool_pda,
        spl_token_program: token_program,
    }
    .instruction()
    .map_err(|e| IndexerError::CustomError(e.to_string()))?;

    Ok(Some(wrap_ix))
}

/// Build instructions for loading compressed accounts (PDAs and/or ctokens).
///
/// Returns instructions in execution order:
/// 1. Wrap instructions for any SPL/T22 ATA balances (separate instructions)
/// 2. ONE decompress instruction for all compressed accounts (PDAs + ctokens together)
///
/// The on-chain `decompress_accounts_idempotent` handler processes both PDAs
/// and ctokens in a single instruction. For this to work, `T` must be a variant
/// enum that includes both PDA types and `PackedCTokenData<V>`.
///
/// # Arguments
/// * `rpc` - RPC client with indexer
/// * `program_id` - The program that will process the decompress instruction
/// * `discriminator` - Instruction discriminator for decompress_accounts_idempotent
/// * `program_accounts` - All compressed accounts (PDAs AND ctokens) to decompress
/// * `program_account_metas` - Account metas for the program instruction
/// * `payer` - Transaction payer
/// * `owner` - Owner of the ATAs (must be a signer)
/// * `atas` - ATAs to check for SPL/T22 wrap as `(mint, ctoken_ata)` tuples
pub async fn build_load_params<R, T>(
    rpc: &mut R,
    program_id: &Pubkey,
    discriminator: &[u8],
    program_accounts: &[CompressibleAccountInput<T>],
    program_account_metas: &[AccountMeta],
    payer: Pubkey,
    owner: Pubkey,
    atas: &[(Pubkey, Pubkey)], // (mint, ctoken_ata)
) -> Result<Vec<Instruction>, IndexerError>
where
    R: Rpc + Indexer,
    T: Pack + Clone + std::fmt::Debug,
{
    let mut instructions = Vec::new();
    let spl_token_program = Pubkey::new_from_array(SPL_TOKEN_PROGRAM_ID);
    let t22_token_program = Pubkey::new_from_array(SPL_TOKEN_2022_PROGRAM_ID);

    // 1. Create wrap instructions for any SPL/T22 ATA balances
    for (mint, ctoken_ata) in atas {
        // Try SPL first
        if let Some(ix) =
            try_wrap_spl_balance(rpc, &owner, mint, payer, *ctoken_ata, spl_token_program).await?
        {
            instructions.push(ix);
        }
        // Then try T22
        else if let Some(ix) =
            try_wrap_spl_balance(rpc, &owner, mint, payer, *ctoken_ata, t22_token_program).await?
        {
            instructions.push(ix);
        }
    }

    // 2. Filter to only compressed accounts
    let compressed_accounts: Vec<_> = program_accounts
        .iter()
        .filter(|acc| acc.is_compressed())
        .collect();

    // If nothing is compressed, return just the wrap instructions
    if compressed_accounts.is_empty() {
        return Ok(instructions);
    }

    // 3. Collect all hashes for validity proof
    let all_hashes: Vec<[u8; 32]> = compressed_accounts
        .iter()
        .filter_map(|acc| acc.merkle_context().map(|ctx| ctx.hash))
        .collect();

    if all_hashes.is_empty() {
        return Ok(instructions);
    }

    // 4. Make ONE validity proof request for all hashes
    let validity_proof_response = rpc
        .get_validity_proof(all_hashes.clone(), vec![], None)
        .await?;
    let validity_proof = validity_proof_response.value;

    // 5. Build compressed accounts with data for decompress instruction
    let compressed_accounts_with_data: Vec<(CompressedAccount, T)> = compressed_accounts
        .iter()
        .map(|acc| {
            let ctx = acc.merkle_context().unwrap();
            let compressed_account = CompressedAccount {
                address: None,
                data: None,
                hash: ctx.hash,
                lamports: acc.info.account_info.lamports,
                leaf_index: ctx.leaf_index,
                owner: acc.info.account_info.owner,
                tree_info: ctx.tree_info.clone(),
                prove_by_index: ctx.prove_by_index,
                seq: None,
                slot_created: 0,
            };
            (compressed_account, acc.parsed.clone())
        })
        .collect();

    let addresses: Vec<_> = compressed_accounts.iter().map(|acc| acc.address).collect();

    // 6. Create ONE decompress instruction (handles both PDAs and ctokens)
    let decompress_ix = decompress_accounts_idempotent(
        program_id,
        discriminator,
        &addresses,
        &compressed_accounts_with_data,
        program_account_metas,
        validity_proof,
    )
    .map_err(|e| IndexerError::CustomError(e.to_string()))?;

    instructions.push(decompress_ix);

    Ok(instructions)
}

#[cfg(test)]
mod tests {
    use light_client::indexer::TreeInfo;
    use light_compressed_account::TreeType;
    use solana_account::Account;

    use super::*;

    fn make_pubkey(seed: u8) -> Pubkey {
        Pubkey::new_from_array([seed; 32])
    }

    fn make_tree_info() -> TreeInfo {
        TreeInfo {
            tree: make_pubkey(10),
            queue: make_pubkey(11),
            cpi_context: None,
            next_tree_info: None,
            tree_type: TreeType::StateV2,
        }
    }

    fn make_merkle_context() -> MerkleContext {
        MerkleContext {
            tree_info: make_tree_info(),
            hash: [1u8; 32],
            leaf_index: 42,
            prove_by_index: true,
        }
    }

    fn make_account_info(is_compressed: bool, with_merkle: bool) -> AccountInfoInterface {
        AccountInfoInterface {
            account_info: Account {
                lamports: 1_000_000,
                data: vec![0u8; 100],
                owner: make_pubkey(5),
                executable: false,
                rent_epoch: 0,
            },
            is_compressed,
            merkle_context: if with_merkle {
                Some(make_merkle_context())
            } else {
                None
            },
        }
    }

    #[derive(Clone, Debug)]
    struct MockData {
        value: u64,
    }

    #[test]
    fn test_compressible_account_input_new() {
        let address = make_pubkey(1);
        let info = make_account_info(true, true);
        let parsed = MockData { value: 123 };

        let input = CompressibleAccountInput::new(address, info.clone(), parsed.clone());

        assert_eq!(input.address, address);
        assert_eq!(input.parsed.value, 123);
        assert!(input.is_compressed());
    }

    #[test]
    fn test_compressible_account_input_is_compressed_true() {
        let input = CompressibleAccountInput::new(
            make_pubkey(1),
            make_account_info(true, true),
            MockData { value: 0 },
        );
        assert!(input.is_compressed());
    }

    #[test]
    fn test_compressible_account_input_is_compressed_false() {
        let input = CompressibleAccountInput::new(
            make_pubkey(1),
            make_account_info(false, false),
            MockData { value: 0 },
        );
        assert!(!input.is_compressed());
    }

    #[test]
    fn test_compressible_account_input_merkle_context_some() {
        let input = CompressibleAccountInput::new(
            make_pubkey(1),
            make_account_info(true, true),
            MockData { value: 0 },
        );
        let ctx = input.merkle_context();
        assert!(ctx.is_some());
        let ctx = ctx.unwrap();
        assert_eq!(ctx.leaf_index, 42);
        assert_eq!(ctx.hash, [1u8; 32]);
        assert!(ctx.prove_by_index);
    }

    #[test]
    fn test_compressible_account_input_merkle_context_none() {
        let input = CompressibleAccountInput::new(
            make_pubkey(1),
            make_account_info(false, false),
            MockData { value: 0 },
        );
        assert!(input.merkle_context().is_none());
    }

    #[test]
    fn test_get_spl_ata_deterministic() {
        let owner = make_pubkey(1);
        let mint = make_pubkey(2);
        let program = make_pubkey(3);

        let ata1 = get_spl_ata(&owner, &mint, &program);
        let ata2 = get_spl_ata(&owner, &mint, &program);
        assert_eq!(ata1, ata2);
    }

    #[test]
    fn test_get_spl_ata_different_owners() {
        let owner1 = make_pubkey(1);
        let owner2 = make_pubkey(2);
        let mint = make_pubkey(10);
        let program = make_pubkey(20);

        let ata1 = get_spl_ata(&owner1, &mint, &program);
        let ata2 = get_spl_ata(&owner2, &mint, &program);
        assert_ne!(ata1, ata2);
    }

    #[test]
    fn test_spl_vs_t22_ata_different_addresses() {
        let owner = make_pubkey(1);
        let mint = make_pubkey(2);

        let spl_program = Pubkey::new_from_array(SPL_TOKEN_PROGRAM_ID);
        let t22_program = Pubkey::new_from_array(SPL_TOKEN_2022_PROGRAM_ID);

        let spl_ata = get_spl_ata(&owner, &mint, &spl_program);
        let t22_ata = get_spl_ata(&owner, &mint, &t22_program);

        assert_ne!(spl_ata, t22_ata);
    }
}

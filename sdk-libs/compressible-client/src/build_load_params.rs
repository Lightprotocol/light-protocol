//! Build load params - unified function for loading PDAs + ATAs

use light_client::{
    indexer::{CompressedAccount, Indexer, IndexerError},
    rpc::Rpc,
};
use light_sdk::compressible::Pack;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::{
    compressible_instruction::decompress_accounts_idempotent,
    get_compressible_account::{AccountInfoInterface, MerkleContext},
};

/// Input for build_load_params - a program account with its parsed data
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

/// Build instructions for loading program accounts and ATAs.
/// Returns a flat `Vec<Instruction>`.
pub async fn build_load_params<R, T>(
    rpc: &mut R,
    program_id: &Pubkey,
    discriminator: &[u8],
    program_accounts: &[CompressibleAccountInput<T>],
    program_account_metas: &[AccountMeta],
    ata_instructions: Vec<Instruction>,
) -> Result<Vec<Instruction>, IndexerError>
where
    R: Rpc + Indexer,
    T: Pack + Clone + std::fmt::Debug,
{
    let mut instructions = ata_instructions;

    let compressed_accounts: Vec<_> = program_accounts
        .iter()
        .filter(|acc| acc.is_compressed())
        .collect();

    if compressed_accounts.is_empty() {
        return Ok(instructions);
    }

    let hashes: Vec<[u8; 32]> = compressed_accounts
        .iter()
        .filter_map(|acc| acc.merkle_context().map(|ctx| ctx.hash))
        .collect();

    let validity_proof_response = rpc.get_validity_proof(hashes, vec![], None).await?;
    let validity_proof = validity_proof_response.value;

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
                tree_info: ctx.tree_info,
                prove_by_index: ctx.prove_by_index,
                seq: None,
                slot_created: 0,
            };
            (compressed_account, acc.parsed.clone())
        })
        .collect();

    let addresses: Vec<_> = compressed_accounts.iter().map(|acc| acc.address).collect();

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
    fn test_compressible_account_input_address_field() {
        let address = make_pubkey(42);
        let input = CompressibleAccountInput::new(
            address,
            make_account_info(false, false),
            MockData { value: 0 },
        );
        assert_eq!(input.address, address);
    }

    #[test]
    fn test_compressible_account_input_info_field() {
        let info = make_account_info(true, true);
        let input =
            CompressibleAccountInput::new(make_pubkey(1), info.clone(), MockData { value: 0 });

        assert_eq!(input.info.account_info.lamports, info.account_info.lamports);
        assert_eq!(input.info.is_compressed, info.is_compressed);
    }

    #[test]
    fn test_compressible_account_input_parsed_field() {
        let parsed = MockData { value: 999 };
        let input =
            CompressibleAccountInput::new(make_pubkey(1), make_account_info(false, false), parsed);
        assert_eq!(input.parsed.value, 999);
    }
}

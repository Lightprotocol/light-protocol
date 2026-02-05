use light_compressed_account::{
    compressed_account::{
        CompressedAccount as ProgramCompressedAccount, CompressedAccountData,
        CompressedAccountWithMerkleContext,
    },
    TreeType,
};
use solana_pubkey::Pubkey;
use tracing::warn;

use super::{
    super::{base58::decode_base58_to_fixed_array, tree_info::QUEUE_TREE_MAPPING, IndexerError},
    tree::{NextTreeInfo, TreeInfo},
};

#[derive(Clone, Default, Debug, PartialEq)]
pub struct CompressedAccount {
    pub address: Option<[u8; 32]>,
    pub data: Option<CompressedAccountData>,
    pub hash: [u8; 32],
    pub lamports: u64,
    pub leaf_index: u32,
    pub owner: Pubkey,
    pub prove_by_index: bool,
    pub seq: Option<u64>,
    pub slot_created: u64,
    pub tree_info: TreeInfo,
}

impl TryFrom<CompressedAccountWithMerkleContext> for CompressedAccount {
    type Error = IndexerError;

    fn try_from(account: CompressedAccountWithMerkleContext) -> Result<Self, Self::Error> {
        let hash = account
            .hash()
            .map_err(|e| IndexerError::decode_error("data", e))?;
        // Breaks light-program-test
        let tree_info = QUEUE_TREE_MAPPING.get(
            &Pubkey::new_from_array(account.merkle_context.merkle_tree_pubkey.to_bytes())
                .to_string(),
        );
        let cpi_context = if let Some(tree_info) = tree_info {
            tree_info.cpi_context
        } else {
            warn!("Cpi context not found in queue tree mapping");
            None
        };
        Ok(CompressedAccount {
            address: account.compressed_account.address,
            data: account.compressed_account.data,
            hash,
            lamports: account.compressed_account.lamports,
            leaf_index: account.merkle_context.leaf_index,
            tree_info: TreeInfo {
                tree: Pubkey::new_from_array(account.merkle_context.merkle_tree_pubkey.to_bytes()),
                queue: Pubkey::new_from_array(account.merkle_context.queue_pubkey.to_bytes()),
                tree_type: account.merkle_context.tree_type,
                cpi_context,
                next_tree_info: None,
            },
            owner: Pubkey::new_from_array(account.compressed_account.owner.to_bytes()),
            prove_by_index: account.merkle_context.prove_by_index,
            seq: None,
            slot_created: u64::MAX,
        })
    }
}

impl From<CompressedAccount> for CompressedAccountWithMerkleContext {
    fn from(account: CompressedAccount) -> Self {
        use light_compressed_account::Pubkey;
        let compressed_account = ProgramCompressedAccount {
            owner: Pubkey::new_from_array(account.owner.to_bytes()),
            lamports: account.lamports,
            address: account.address,
            data: account.data,
        };

        let merkle_context = account
            .tree_info
            .to_light_merkle_context(account.leaf_index, account.prove_by_index);

        CompressedAccountWithMerkleContext {
            compressed_account,
            merkle_context,
        }
    }
}

impl TryFrom<&photon_api::models::AccountV2> for CompressedAccount {
    type Error = IndexerError;

    fn try_from(account: &photon_api::models::AccountV2) -> Result<Self, Self::Error> {
        let data = if let Some(data) = &account.data {
            Ok::<Option<CompressedAccountData>, IndexerError>(Some(CompressedAccountData {
                discriminator: data.discriminator.to_le_bytes(),
                data: base64::decode_config(&data.data, base64::STANDARD_NO_PAD)
                    .map_err(|e| IndexerError::decode_error("data", e))?,
                data_hash: decode_base58_to_fixed_array(&data.data_hash)?,
            }))
        } else {
            Ok::<Option<CompressedAccountData>, IndexerError>(None)
        }?;

        let owner = Pubkey::new_from_array(decode_base58_to_fixed_array(&account.owner)?);
        let address = account
            .address
            .as_ref()
            .map(|address| decode_base58_to_fixed_array(address))
            .transpose()?;
        let hash = decode_base58_to_fixed_array(&account.hash)?;

        let tree_info = TreeInfo {
            tree: Pubkey::new_from_array(decode_base58_to_fixed_array(
                &account.merkle_context.tree,
            )?),
            queue: Pubkey::new_from_array(decode_base58_to_fixed_array(
                &account.merkle_context.queue,
            )?),
            tree_type: TreeType::from(account.merkle_context.tree_type as u64),
            cpi_context: super::super::base58::decode_base58_option_to_pubkey(
                &account.merkle_context.cpi_context,
            )?,
            next_tree_info: account
                .merkle_context
                .next_tree_context
                .as_ref()
                .map(|ctx| NextTreeInfo::try_from(ctx.as_ref()))
                .transpose()?,
        };

        Ok(CompressedAccount {
            owner,
            address,
            data,
            hash,
            lamports: account.lamports,
            leaf_index: account.leaf_index,
            seq: account.seq,
            slot_created: account.slot_created,
            tree_info,
            prove_by_index: account.prove_by_index,
        })
    }
}

impl TryFrom<&photon_api::models::Account> for CompressedAccount {
    type Error = IndexerError;

    fn try_from(account: &photon_api::models::Account) -> Result<Self, Self::Error> {
        let data = if let Some(data) = &account.data {
            Ok::<Option<CompressedAccountData>, IndexerError>(Some(CompressedAccountData {
                discriminator: data.discriminator.to_le_bytes(),
                data: base64::decode_config(&data.data, base64::STANDARD_NO_PAD)
                    .map_err(|e| IndexerError::decode_error("data", e))?,
                data_hash: decode_base58_to_fixed_array(&data.data_hash)?,
            }))
        } else {
            Ok::<Option<CompressedAccountData>, IndexerError>(None)
        }?;
        let owner = Pubkey::new_from_array(decode_base58_to_fixed_array(&account.owner)?);
        let address = account
            .address
            .as_ref()
            .map(|address| decode_base58_to_fixed_array(address))
            .transpose()?;
        let hash = decode_base58_to_fixed_array(&account.hash)?;
        let seq = account.seq;
        let slot_created = account.slot_created;
        let lamports = account.lamports;
        let leaf_index = account.leaf_index;

        let tree_info =
            QUEUE_TREE_MAPPING
                .get(&account.tree)
                .ok_or(IndexerError::MissingResult {
                    context: "conversion".into(),
                    message: "expected value was None".into(),
                })?;

        let tree_info = TreeInfo {
            cpi_context: tree_info.cpi_context,
            queue: tree_info.queue,
            tree_type: tree_info.tree_type,
            next_tree_info: None,
            tree: tree_info.tree,
        };

        Ok(CompressedAccount {
            owner,
            address,
            data,
            hash,
            lamports,
            leaf_index,
            seq,
            slot_created,
            tree_info,
            prove_by_index: false,
        })
    }
}

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use solana_program::pubkey::Pubkey;

use crate::instruction_accounts::LightInstructionAccounts;

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct QueueIndex {
    /// Id of queue in queue account.
    pub queue_id: u8,
    /// Index of compressed account hash in queue.
    pub index: u16,
}

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct MerkleContext {
    pub merkle_tree_pubkey: Pubkey,
    pub nullifier_queue_pubkey: Pubkey,
    pub leaf_index: u32,
    /// Index of leaf in queue. Placeholder of batched Merkle tree updates
    /// currently unimplemented.
    pub queue_index: Option<QueueIndex>,
}

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct PackedMerkleContext {
    pub merkle_tree_pubkey_index: u8,
    pub nullifier_queue_pubkey_index: u8,
    pub leaf_index: u32,
    /// Index of leaf in queue. Placeholder of batched Merkle tree updates
    /// currently unimplemented.
    pub queue_index: Option<QueueIndex>,
}

pub fn pack_merkle_contexts<'a, I>(
    merkle_contexts: I,
    remaining_accounts: &'a mut LightInstructionAccounts,
) -> impl Iterator<Item = PackedMerkleContext> + 'a
where
    I: Iterator<Item = &'a MerkleContext> + 'a,
{
    merkle_contexts.map(|x| pack_merkle_context(x, remaining_accounts))
}

pub fn pack_merkle_context(
    merkle_context: &MerkleContext,
    remaining_accounts: &mut LightInstructionAccounts,
) -> PackedMerkleContext {
    let MerkleContext {
        merkle_tree_pubkey,
        nullifier_queue_pubkey,
        leaf_index,
        queue_index,
    } = merkle_context;
    let merkle_tree_pubkey_index = remaining_accounts.insert_or_get(*merkle_tree_pubkey);
    let nullifier_queue_pubkey_index = remaining_accounts.insert_or_get(*nullifier_queue_pubkey);

    PackedMerkleContext {
        merkle_tree_pubkey_index,
        nullifier_queue_pubkey_index,
        leaf_index: *leaf_index,
        queue_index: *queue_index,
    }
}

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct AddressMerkleContext {
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_queue_pubkey: Pubkey,
}

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct PackedAddressMerkleContext {
    pub address_merkle_tree_pubkey_index: u8,
    pub address_queue_pubkey_index: u8,
}

/// Returns an iterator of [`PackedAddressMerkleContext`] and fills up
/// `remaining_accounts` based on the given `merkle_contexts`.
pub fn pack_address_merkle_contexts<'a, I>(
    address_merkle_contexts: I,
    remaining_accounts: &'a mut LightInstructionAccounts,
) -> impl Iterator<Item = PackedAddressMerkleContext> + 'a
where
    I: Iterator<Item = &'a AddressMerkleContext> + 'a,
{
    address_merkle_contexts.map(|x| pack_address_merkle_context(x, remaining_accounts))
}

/// Returns a [`PackedAddressMerkleContext`] and fills up `remaining_accounts`
/// based on the given `merkle_context`.
pub fn pack_address_merkle_context(
    address_merkle_context: &AddressMerkleContext,
    remaining_accounts: &mut LightInstructionAccounts,
) -> PackedAddressMerkleContext {
    let AddressMerkleContext {
        address_merkle_tree_pubkey,
        address_queue_pubkey,
    } = address_merkle_context;
    let address_merkle_tree_pubkey_index =
        remaining_accounts.insert_or_get(*address_merkle_tree_pubkey);
    let address_queue_pubkey_index = remaining_accounts.insert_or_get(*address_queue_pubkey);

    PackedAddressMerkleContext {
        address_merkle_tree_pubkey_index,
        address_queue_pubkey_index,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::instruction_accounts::LightInstructionAccounts;

    #[test]
    fn test_pack_merkle_context() {
        let registered_program_pda = Pubkey::new_unique();
        let account_compression_authority = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        let mut accounts = LightInstructionAccounts::new(
            &registered_program_pda,
            &account_compression_authority,
            &program_id,
            None,
            None,
        );

        let merkle_tree_pubkey = Pubkey::new_unique();
        let nullifier_queue_pubkey = Pubkey::new_unique();
        let merkle_context = MerkleContext {
            merkle_tree_pubkey,
            nullifier_queue_pubkey,
            leaf_index: 69,
            queue_index: None,
        };

        let packed_merkle_context = pack_merkle_context(&merkle_context, &mut accounts);
        assert_eq!(
            packed_merkle_context,
            PackedMerkleContext {
                merkle_tree_pubkey_index: 9,
                nullifier_queue_pubkey_index: 10,
                leaf_index: 69,
                queue_index: None,
            }
        )
    }

    #[test]
    fn test_pack_merkle_contexts() {
        let registered_program_pda = Pubkey::new_unique();
        let account_compression_authority = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        let mut accounts = LightInstructionAccounts::new(
            &registered_program_pda,
            &account_compression_authority,
            &program_id,
            None,
            None,
        );

        let merkle_contexts = &[
            MerkleContext {
                merkle_tree_pubkey: Pubkey::new_unique(),
                nullifier_queue_pubkey: Pubkey::new_unique(),
                leaf_index: 10,
                queue_index: None,
            },
            MerkleContext {
                merkle_tree_pubkey: Pubkey::new_unique(),
                nullifier_queue_pubkey: Pubkey::new_unique(),
                leaf_index: 11,
                queue_index: Some(QueueIndex {
                    queue_id: 69,
                    index: 420,
                }),
            },
            MerkleContext {
                merkle_tree_pubkey: Pubkey::new_unique(),
                nullifier_queue_pubkey: Pubkey::new_unique(),
                leaf_index: 12,
                queue_index: None,
            },
        ];

        let packed_merkle_contexts = pack_merkle_contexts(merkle_contexts.iter(), &mut accounts);
        assert_eq!(
            packed_merkle_contexts.collect::<Vec<_>>(),
            &[
                PackedMerkleContext {
                    merkle_tree_pubkey_index: 9,
                    nullifier_queue_pubkey_index: 10,
                    leaf_index: 10,
                    queue_index: None
                },
                PackedMerkleContext {
                    merkle_tree_pubkey_index: 11,
                    nullifier_queue_pubkey_index: 12,
                    leaf_index: 11,
                    queue_index: Some(QueueIndex {
                        queue_id: 69,
                        index: 420
                    })
                },
                PackedMerkleContext {
                    merkle_tree_pubkey_index: 13,
                    nullifier_queue_pubkey_index: 14,
                    leaf_index: 12,
                    queue_index: None,
                }
            ]
        );
    }

    #[test]
    fn test_pack_address_merkle_context() {
        let registered_program_pda = Pubkey::new_unique();
        let account_compression_authority = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        let mut accounts = LightInstructionAccounts::new(
            &registered_program_pda,
            &account_compression_authority,
            &program_id,
            None,
            None,
        );

        let address_merkle_context = AddressMerkleContext {
            address_merkle_tree_pubkey: Pubkey::new_unique(),
            address_queue_pubkey: Pubkey::new_unique(),
        };

        let packed_address_merkle_context =
            pack_address_merkle_context(&address_merkle_context, &mut accounts);
        assert_eq!(
            packed_address_merkle_context,
            PackedAddressMerkleContext {
                address_merkle_tree_pubkey_index: 9,
                address_queue_pubkey_index: 10,
            }
        )
    }

    #[test]
    fn test_pack_address_merkle_contexts() {
        let registered_program_pda = Pubkey::new_unique();
        let account_compression_authority = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        let mut accounts = LightInstructionAccounts::new(
            &registered_program_pda,
            &account_compression_authority,
            &program_id,
            None,
            None,
        );

        let address_merkle_contexts = &[
            AddressMerkleContext {
                address_merkle_tree_pubkey: Pubkey::new_unique(),
                address_queue_pubkey: Pubkey::new_unique(),
            },
            AddressMerkleContext {
                address_merkle_tree_pubkey: Pubkey::new_unique(),
                address_queue_pubkey: Pubkey::new_unique(),
            },
            AddressMerkleContext {
                address_merkle_tree_pubkey: Pubkey::new_unique(),
                address_queue_pubkey: Pubkey::new_unique(),
            },
        ];

        let packed_address_merkle_contexts =
            pack_address_merkle_contexts(address_merkle_contexts.iter(), &mut accounts);
        assert_eq!(
            packed_address_merkle_contexts.collect::<Vec<_>>(),
            &[
                PackedAddressMerkleContext {
                    address_merkle_tree_pubkey_index: 9,
                    address_queue_pubkey_index: 10,
                },
                PackedAddressMerkleContext {
                    address_merkle_tree_pubkey_index: 11,
                    address_queue_pubkey_index: 12,
                },
                PackedAddressMerkleContext {
                    address_merkle_tree_pubkey_index: 13,
                    address_queue_pubkey_index: 14,
                }
            ]
        );
    }
}

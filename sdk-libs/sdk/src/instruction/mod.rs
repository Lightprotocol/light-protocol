//! # Instruction data with AccountMeta (in Solana program)
//! ### Example instruction data to create new compressed account with address:
//! ```ignore
//! #[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
//! pub struct CreatePdaInstructionData {
//!    /// Prove validity of the new address.
//!    pub proof: ValidityProof,
//!    pub address_tree_info: PackedAddressTreeInfo,
//!    /// Index of Merkle tree in the account array (remaining accounts in anchor).
//!    pub output_merkle_tree_index: u8,
//!    /// Arbitrary data of the new account.
//!    pub data: [u8; 31],
//!    /// Account offsets for convenience (can be hardcoded).
//!    pub system_accounts_offset: u8,
//!    pub tree_accounts_offset: u8,
//! }
//! ```
//!
//!
//! ### Example instruction data to update a compressed account:
//! ```ignore
//! #[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
//!pub struct UpdatePdaInstructionData {
//!    /// Prove validity of the existing compressed account state.
//!    pub proof: ValidityProof,
//!    /// Data and metadata of the compressed account.
//!    pub my_compressed_account: UpdateMyCompressedAccount,
//!    /// Arbitrary new data the compressed account will be updated with.
//!    pub new_data: [u8; 31],
//!    /// Account offsets for convenience (can be hardcoded).
//!    pub system_accounts_offset: u8,
//! }
//!
//! #[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
//! pub struct UpdateMyCompressedAccount {
//!     /// Metadata of the compressed account.
//!     pub meta: CompressedAccountMeta,
//!     /// Data of the compressed account.
//!     pub data: [u8; 31],
//! }
//! ```
//! ### Example anchor instruction data to create new compressed account with address:
//! ```ignore
//! pub fn create_compressed_account<'info>(
//!     ctx: Context<'_, '_, '_, 'info, WithNestedData<'info>>,
//!     /// Prove validity of the new address.
//!     proof: ValidityProof,
//!     address_tree_info: PackedAddressTreeInfo,
//!     /// Index of Merkle tree in remaining accounts.
//!     output_tree_index: u8,
//!     /// Arbitrary data of the new account.
//!     name: String,
//! ) -> Result<()>;
//! ```
//! ### Example anchor instruction data to update a compressed account:
//! ```ignore
//! pub fn update_compressed_account<'info>(
//!     ctx: Context<'_, '_, '_, 'info, UpdateNestedData<'info>>,
//!     /// Prove validity of the existing compressed account state.
//!     proof: ValidityProof,
//!     /// Data of the compressed account.
//!     my_compressed_account: MyCompressedAccount,
//!     /// Metadata of the compressed account.
//!     account_meta: CompressedAccountMeta,
//!     /// Arbitrary new data the compressed account will be updated with.
//!     nested_data: NestedData,
//! ) -> Result<()>;
//! ```
//! ### Example instruction data to update a compressed account:
//! # Create instruction with packed accounts (in client)
//!
//! ### Create instruction to create 1 compressed account and address
//! ```ignore
//! let config =
//!    ProgramTestConfig::new_v2(true, Some(vec![("sdk_anchor_test", sdk_anchor_test::ID)]));
//! let mut rpc = LightProgramTest::new(config).await.unwrap();
//! let name = "test";
//! let address_tree_info = rpc.get_address_tree_v1();
//! let (address, _) = derive_address(
//!    &[b"compressed", name.as_bytes()],
//!    &address_tree_info.tree,
//!    &sdk_anchor_test::ID,
//! );
//! let config = SystemAccountMetaConfig::new(sdk_anchor_test::ID);
//! let mut remaining_accounts = PackedAccounts::default();
//! remaining_accounts.add_system_accounts(config);
//!
//! let address_merkle_tree_info = rpc.get_address_tree_v1();
//!
//! let rpc_result = rpc
//!     .get_validity_proof(
//!         vec![],
//!         vec![AddressWithTree {
//!             address: *address,
//!             tree: address_merkle_tree_info.tree,
//!         }],
//!         None,
//!     )
//!     .await?
//!     .value;
//! let packed_accounts = rpc_result.pack_tree_infos(&mut remaining_accounts);
//!
//! let output_tree_index = rpc
//!     .get_random_state_tree_info()
//!     .pack_output_tree_index(&mut remaining_accounts)
//!     .unwrap();
//!
//! let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();
//!
//! let instruction_data = sdk_anchor_test::instruction::WithNestedData {
//!     proof: rpc_result.proof,
//!     address_tree_info: packed_accounts.address_trees[0],
//!     name,
//!     output_tree_index,
//! };
//!
//! let accounts = sdk_anchor_test::accounts::WithNestedData {
//!     signer: payer.pubkey(),
//! };
//!
//! let instruction = Instruction {
//!     program_id: sdk_anchor_test::ID,
//!     accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
//!     data: instruction_data.data(),
//! };
//! ```
//!
//! ### Create instruction to create 1 compressed account and address (anchor)
//! ```ignore
//! let mut remaining_accounts = PackedAccounts::default();
//!
//! let config = SystemAccountMetaConfig::new(sdk_anchor_test::ID);
//! remaining_accounts.add_system_accounts(config);
//! let hash = compressed_account.hash;
//!
//! let rpc_result = rpc
//!     .get_validity_proof(vec![hash], vec![], None)
//!     .await?
//!     .value;
//!
//! let packed_tree_accounts = rpc_result
//!     .pack_tree_infos(&mut remaining_accounts)
//!     .state_trees
//!     .unwrap();
//!
//! let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();
//!
//! let my_compressed_account = MyCompressedAccount::deserialize(
//!     &mut compressed_account.data.as_mut().unwrap().data.as_slice(),
//! )
//! .unwrap();
//! let instruction_data = sdk_anchor_test::instruction::UpdateNestedData {
//!     proof: rpc_result.proof,
//!     my_compressed_account,
//!     account_meta: CompressedAccountMeta {
//!         tree_info: packed_tree_accounts.packed_tree_infos[0],
//!         address: compressed_account.address.unwrap(),
//!         output_state_tree_index: packed_tree_accounts.output_tree_index,
//!     },
//!     nested_data,
//! };
//!
//! let accounts = sdk_anchor_test::accounts::UpdateNestedData {
//!     signer: payer.pubkey(),
//! };
//!
//! let instruction = Instruction {
//!     program_id: sdk_anchor_test::ID,
//!     accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
//!     data: instruction_data.data(),
//! };
//! ```
// TODO: link to examples

mod pack_accounts;
mod system_accounts;
mod tree_info;

/// Zero-knowledge proof to prove the validity of existing compressed accounts and new addresses.
pub use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
pub use light_sdk_types::instruction::*;
pub use pack_accounts::*;
pub use system_accounts::*;
pub use tree_info::*;

// //! Comprehensive randomized test for CompressedToken zero-copy methods
// //! Tests zero_copy_at, zero_copy_at_mut, new_zero_copy and setter methods in 1k iterations
// //!

// use light_compressed_account::Pubkey;
// use light_ctoken_types::state::{
//     extensions::ExtensionStructConfig,
//     solana_ctoken::{CompressedToken, CompressedTokenConfig},
//     CompressionInfoConfig, ZExtensionStruct, ZExtensionStructMut,
// };
// use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
// use rand::{distributions::Standard, prelude::Distribution, Rng};
// use spl_token_2022::{
//     solana_program::program_pack::Pack,
//     state::{Account, AccountState},
// };

// #[derive(Clone, Debug)]
// struct RandomTokenData {
//     mint: Pubkey,
//     owner: Pubkey,
//     amount: u64,
//     delegate: Option<Pubkey>,
//     state: u8,
//     is_native: Option<u64>,
//     delegated_amount: u64,
//     close_authority: Option<Pubkey>,
//     has_extensions: bool,
//     // Extension data
//     last_written_slot: u64,
//     slots_until_compression: u64,
//     compression_authority: Pubkey,
//     rent_sponsor: Pubkey,
// }

// impl Distribution<RandomTokenData> for Standard {
//     fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> RandomTokenData {
//         RandomTokenData {
//             mint: rng.gen::<[u8; 32]>().into(),
//             owner: rng.gen::<[u8; 32]>().into(),
//             amount: rng.gen::<u64>(),
//             delegate: if rng.gen_bool(0.3) {
//                 Some(rng.gen::<[u8; 32]>().into())
//             } else {
//                 None
//             },
//             state: rng.gen_range(0..=2),
//             is_native: if rng.gen_bool(0.2) {
//                 Some(rng.gen_range(1_000_000..=10_000_000))
//             } else {
//                 None
//             },
//             delegated_amount: rng.gen::<u64>(),
//             close_authority: if rng.gen_bool(0.25) {
//                 Some(rng.gen::<[u8; 32]>().into())
//             } else {
//                 None
//             },
//             has_extensions: rng.gen_bool(0.3),
//             // Extension data
//             last_written_slot: rng.gen::<u64>(),
//             slots_until_compression: rng.gen::<u64>(),
//             compression_authority: rng.gen::<[u8; 32]>().into(),
//             rent_sponsor: rng.gen::<[u8; 32]>().into(),
//         }
//     }
// }

// fn create_spl_data(data: &RandomTokenData) -> Vec<u8> {
//     let account = Account {
//         mint: solana_pubkey::Pubkey::new_from_array(data.mint.to_bytes()),
//         owner: solana_pubkey::Pubkey::new_from_array(data.owner.to_bytes()),
//         amount: data.amount,
//         delegate: data
//             .delegate
//             .map(|d| {
//                 spl_pod::solana_program_option::COption::Some(
//                     solana_pubkey::Pubkey::new_from_array(d.to_bytes()),
//                 )
//             })
//             .unwrap_or(spl_pod::solana_program_option::COption::None),
//         state: match data.state {
//             0 => AccountState::Uninitialized,
//             1 => AccountState::Initialized,
//             2 => AccountState::Frozen,
//             _ => AccountState::Initialized,
//         },
//         is_native: data
//             .is_native
//             .map(spl_pod::solana_program_option::COption::Some)
//             .unwrap_or(spl_pod::solana_program_option::COption::None),
//         delegated_amount: data.delegated_amount,
//         close_authority: data
//             .close_authority
//             .map(|ca| {
//                 spl_pod::solana_program_option::COption::Some(
//                     solana_pubkey::Pubkey::new_from_array(ca.to_bytes()),
//                 )
//             })
//             .unwrap_or(spl_pod::solana_program_option::COption::None),
//     };

//     let mut account_data = vec![0u8; Account::LEN];
//     Account::pack(account, &mut account_data).unwrap();

//     if data.has_extensions {
//         account_data.push(2u8); // AccountType::Account
//         account_data.push(1u8); // Some extensions
//         account_data.extend_from_slice(&1u32.to_le_bytes()); // Vec length = 1
//         account_data.push(26u8); // Compressible discriminant
//                                  // CompressionInfo: last_written_slot(8) + slots_until_compression(8) + compression_authority(32) + rent_sponsor(32) = 80 bytes
//         account_data.extend_from_slice(&data.last_written_slot.to_le_bytes());
//         account_data.extend_from_slice(&data.slots_until_compression.to_le_bytes());
//         account_data.extend_from_slice(&data.compression_authority.to_bytes());
//         account_data.extend_from_slice(&data.rent_sponsor.to_bytes());
//     }

//     account_data
// }

// fn create_config(data: &RandomTokenData) -> CompressedTokenConfig {
//     CompressedTokenConfig {
//         delegate: data.delegate.is_some(),
//         is_native: data.is_native.is_some(),
//         close_authority: data.close_authority.is_some(),
//         extensions: if data.has_extensions {
//             vec![ExtensionStructConfig::Compressible(
//                 CompressionInfoConfig {
//                     lamports_per_write: true,
//                     compression_authority: (true, ()),
//                     rent_sponsor: (true, ()),
//                 },
//             )]
//         } else {
//             vec![]
//         },
//     }
// }

// #[test]
// fn test_zero_copy_randomized() {
//     let mut rng = rand::thread_rng();

//     for iteration in 0..1000 {
//         let data: RandomTokenData = rng.gen();
//         let mut account_data = create_spl_data(&data);

//         // Test zero_copy_at
//         {
//             let (zc_token, remaining) = CompressedToken::zero_copy_at(&account_data).unwrap();
//             assert_eq!(remaining.len(), 0);
//             assert_eq!(zc_token.mint.to_bytes(), data.mint.to_bytes());
//             assert_eq!(zc_token.owner.to_bytes(), data.owner.to_bytes());
//             assert_eq!(u64::from(*zc_token.amount), data.amount);
//             assert_eq!(zc_token.state, data.state);
//             assert_eq!(u64::from(*zc_token.delegated_amount), data.delegated_amount);
//             assert_eq!(zc_token.extensions.is_some(), data.has_extensions);

//             // Verify optional fields
//             match (zc_token.delegate, &data.delegate) {
//                 (Some(zc_del), Some(data_del)) => {
//                     assert_eq!(zc_del.to_bytes(), data_del.to_bytes())
//                 }
//                 (None, None) => {}
//                 _ => panic!("Iteration {}: delegate mismatch", iteration),
//             }

//             match (zc_token.is_native, &data.is_native) {
//                 (Some(zc_native), Some(data_native)) => {
//                     assert_eq!(u64::from(*zc_native), *data_native)
//                 }
//                 (None, None) => {}
//                 _ => panic!("Iteration {}: is_native mismatch", iteration),
//             }

//             match (zc_token.close_authority, &data.close_authority) {
//                 (Some(zc_close), Some(data_close)) => {
//                     assert_eq!(zc_close.to_bytes(), data_close.to_bytes())
//                 }
//                 (None, None) => {}
//                 _ => panic!("Iteration {}: close_authority mismatch", iteration),
//             }
//             if let Some(extension) = zc_token.extensions.as_ref() {
//                 assert_eq!(extension.len(), 1);
//                 match &extension[0] {
//                     ZExtensionStruct::Compressible(e) => {
//                         assert_eq!(u64::from(e.last_written_slot), data.last_written_slot);
//                         assert_eq!(e.compression_authority.to_bytes(), data.compression_authority.to_bytes());
//                         assert_eq!(e.rent_sponsor.to_bytes(), data.rent_sponsor.to_bytes());
//                         assert_eq!(
//                             u64::from(e.slots_until_compression),
//                             data.slots_until_compression
//                         );
//                     }
//                     _ => panic!("Invalid extension"),
//                 }
//             } else if data.has_extensions {
//                 panic!("should have extensions");
//             }
//         }
//         {
//             let (zc_token, remaining) =
//                 CompressedToken::zero_copy_at_mut(&mut account_data).unwrap();
//             assert_eq!(remaining.len(), 0);
//             assert_eq!(zc_token.mint.to_bytes(), data.mint.to_bytes());
//             assert_eq!(zc_token.owner.to_bytes(), data.owner.to_bytes());
//             assert_eq!(u64::from(*zc_token.amount), data.amount);
//             assert_eq!(*zc_token.state, data.state);
//             assert_eq!(u64::from(*zc_token.delegated_amount), data.delegated_amount);
//             assert_eq!(zc_token.extensions.is_some(), data.has_extensions);

//             // Verify optional fields
//             match (zc_token.delegate.as_ref(), &data.delegate) {
//                 (Some(zc_del), Some(data_del)) => {
//                     assert_eq!(zc_del.to_bytes(), data_del.to_bytes())
//                 }
//                 (None, None) => {}
//                 _ => panic!("Iteration {}: delegate mismatch", iteration),
//             }

//             match (zc_token.is_native.as_ref(), &data.is_native) {
//                 (Some(zc_native), Some(data_native)) => {
//                     assert_eq!(u64::from(**zc_native), *data_native)
//                 }
//                 (None, None) => {}
//                 _ => panic!("Iteration {}: is_native mismatch", iteration),
//             }

//             match (zc_token.close_authority.as_ref(), &data.close_authority) {
//                 (Some(zc_close), Some(data_close)) => {
//                     assert_eq!(zc_close.to_bytes(), data_close.to_bytes())
//                 }
//                 (None, None) => {}
//                 _ => panic!("Iteration {}: close_authority mismatch", iteration),
//             }
//             if let Some(extension) = zc_token.extensions.as_ref() {
//                 assert_eq!(extension.len(), 1);
//                 match &extension[0] {
//                     ZExtensionStructMut::Compressible(e) => {
//                         assert_eq!(u64::from(e.last_written_slot), data.last_written_slot);
//                         assert_eq!(e.compression_authority.to_bytes(), data.compression_authority.to_bytes());
//                         assert_eq!(e.rent_sponsor.to_bytes(), data.rent_sponsor.to_bytes());
//                         assert_eq!(
//                             u64::from(e.slots_until_compression),
//                             data.slots_until_compression
//                         );
//                     }
//                     _ => panic!("Invalid extension"),
//                 }
//             } else if data.has_extensions {
//                 panic!("should have extensions");
//             }
//         }
//     }
// }

// #[test]
// fn test_zero_copy_mutate_randomized() {
//     let mut rng = rand::thread_rng();

//     for iteration in 0..1000 {
//         let data: RandomTokenData = rng.gen();
//         let account_data = create_spl_data(&data);

//         // Test zero_copy_at
//         let (zc_token, remaining) = CompressedToken::zero_copy_at(&account_data).unwrap();
//         assert_eq!(remaining.len(), 0);
//         assert_eq!(zc_token.mint.to_bytes(), data.mint.to_bytes());
//         assert_eq!(zc_token.owner.to_bytes(), data.owner.to_bytes());
//         assert_eq!(u64::from(*zc_token.amount), data.amount);
//         assert_eq!(zc_token.state, data.state);
//         assert_eq!(u64::from(*zc_token.delegated_amount), data.delegated_amount);
//         assert_eq!(zc_token.extensions.is_some(), data.has_extensions);

//         // Verify optional fields
//         match (zc_token.delegate, &data.delegate) {
//             (Some(zc_del), Some(data_del)) => assert_eq!(zc_del.to_bytes(), data_del.to_bytes()),
//             (None, None) => {}
//             _ => panic!("Iteration {}: delegate mismatch", iteration),
//         }

//         match (zc_token.is_native, &data.is_native) {
//             (Some(zc_native), Some(data_native)) => assert_eq!(u64::from(*zc_native), *data_native),
//             (None, None) => {}
//             _ => panic!("Iteration {}: is_native mismatch", iteration),
//         }

//         match (zc_token.close_authority, &data.close_authority) {
//             (Some(zc_close), Some(data_close)) => {
//                 assert_eq!(zc_close.to_bytes(), data_close.to_bytes())
//             }
//             (None, None) => {}
//             _ => panic!("Iteration {}: close_authority mismatch", iteration),
//         }

//         // Test zero_copy_at_mut with mutations
//         let mut account_data_mut = account_data.clone();
//         let new_state = rng.gen_range(0..=2);
//         let new_delegate = if rng.gen_bool(0.5) {
//             Some(rng.gen::<[u8; 32]>().into())
//         } else {
//             None
//         };
//         let new_is_native = if rng.gen_bool(0.5) {
//             Some(rng.gen::<u64>())
//         } else {
//             None
//         };
//         let new_close_authority = if rng.gen_bool(0.5) {
//             Some(rng.gen::<[u8; 32]>().into())
//         } else {
//             None
//         };
//         let new_mint = rng.gen::<[u8; 32]>().into();
//         let new_owner = rng.gen::<[u8; 32]>().into();
//         let new_amount = rng.gen::<u64>().into();
//         let new_delegated_amount = rng.gen::<u64>().into();
//         {
//             let (mut zc_token_mut, _) =
//                 CompressedToken::zero_copy_at_mut(&mut account_data_mut).unwrap();

//             // Test mutations
//             *zc_token_mut.mint = new_mint;
//             *zc_token_mut.owner = new_owner;
//             *zc_token_mut.amount = new_amount;
//             *zc_token_mut.state = new_state;
//             *zc_token_mut.delegated_amount = new_delegated_amount;

//             zc_token_mut.set_delegate(new_delegate).unwrap();
//             zc_token_mut.set_is_native(new_is_native).unwrap();
//             zc_token_mut
//                 .set_close_authority(new_close_authority)
//                 .unwrap();
//         }

//         // Verify mutations persisted by re-parsing
//         {
//             let (zc_token_after, _) = CompressedToken::zero_copy_at(&account_data_mut).unwrap();
//             assert_eq!(zc_token_after.mint.to_bytes(), new_mint.to_bytes());
//             assert_eq!(zc_token_after.owner.to_bytes(), new_owner.to_bytes());
//             assert_eq!(*zc_token_after.amount, new_amount);
//             assert_eq!(zc_token_after.state, new_state);
//             assert_eq!(*zc_token_after.delegated_amount, new_delegated_amount);
//         }

//         // Test new_zero_copy round-trip
//         let config = create_config(&data);
//         let required_size = CompressedToken::byte_len(&config).unwrap();
//         let mut buffer = vec![0u8; required_size];

//         {
//             let (zc_new_token, remaining) =
//                 CompressedToken::new_zero_copy(&mut buffer, config.clone()).unwrap();
//             assert_eq!(remaining.len(), 0);
//             assert_eq!(*zc_new_token.state, 1); // Should be initialized
//             assert_eq!(zc_new_token.delegate.is_some(), config.delegate);
//             assert_eq!(zc_new_token.is_native.is_some(), config.is_native);
//             assert_eq!(
//                 zc_new_token.close_authority.is_some(),
//                 config.close_authority
//             );
//             assert_eq!(
//                 zc_new_token.extensions.is_some(),
//                 !config.extensions.is_empty()
//             );
//         }
//         // Verify new_zero_copy result can be re-parsed
//         let (zc_reparsed, _) = CompressedToken::zero_copy_at(&buffer).unwrap();
//         assert_eq!(zc_reparsed.state, 1);
//         assert_eq!(zc_reparsed.delegate.is_some(), config.delegate);
//         assert_eq!(zc_reparsed.is_native.is_some(), config.is_native);
//         assert_eq!(
//             zc_reparsed.close_authority.is_some(),
//             config.close_authority
//         );

//         // Test PartialEq implementation (only for tokens without extensions for simplicity)
//         if !data.has_extensions {
//             let regular_token = CompressedToken {
//                 mint: data.mint,
//                 owner: data.owner,
//                 amount: data.amount,
//                 delegate: data.delegate,
//                 state: data.state,
//                 is_native: data.is_native,
//                 delegated_amount: data.delegated_amount,
//                 close_authority: data.close_authority,
//                 extensions: None,
//             };

//             assert_eq!(zc_token, regular_token);
//             assert_eq!(regular_token, zc_token);
//         }
//     }
// }

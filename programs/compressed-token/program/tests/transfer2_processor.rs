use anchor_lang::AnchorSerialize;
use light_compressed_token::compressed_token::transfer2::processor::is_idempotent_ata_decompress;
use light_token_interface::instructions::{
    extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
    transfer2::{
        CompressedTokenInstructionDataTransfer2, Compression, CompressionMode,
        MultiInputTokenDataWithContext,
    },
};
use light_zero_copy::traits::ZeroCopyAt;
use rand::{rngs::StdRng, Rng, SeedableRng};

fn serialize(data: &CompressedTokenInstructionDataTransfer2) -> Vec<u8> {
    let mut buf = Vec::new();
    AnchorSerialize::serialize(data, &mut buf).unwrap();
    buf
}

fn base_input() -> MultiInputTokenDataWithContext {
    MultiInputTokenDataWithContext::default()
}

fn base_data() -> CompressedTokenInstructionDataTransfer2 {
    CompressedTokenInstructionDataTransfer2 {
        with_transaction_hash: false,
        with_lamports_change_account_merkle_tree_index: false,
        lamports_change_account_merkle_tree_index: 0,
        lamports_change_account_owner_index: 0,
        output_queue: 0,
        max_top_up: 0,
        cpi_context: None,
        compressions: None,
        proof: None,
        in_token_data: vec![],
        out_token_data: vec![],
        in_lamports: None,
        out_lamports: None,
        in_tlv: None,
        out_tlv: None,
    }
}

fn check(data: &CompressedTokenInstructionDataTransfer2) -> bool {
    let buf = serialize(data);
    let (z, _) = CompressedTokenInstructionDataTransfer2::zero_copy_at(&buf).unwrap();
    is_idempotent_ata_decompress(&z)
}

#[test]
fn test_is_idempotent_ata_decompress_empty() {
    assert!(!check(&base_data()));
}

#[test]
fn test_is_idempotent_ata_decompress_no_compressions() {
    let mut data = base_data();
    data.in_token_data = vec![base_input()];
    data.in_tlv = Some(vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0,
            is_ata: true,
            bump: 0,
            owner_index: 0,
        },
    )]]);
    assert!(!check(&data));
}

#[test]
fn test_is_idempotent_ata_decompress_multiple_inputs() {
    let mut data = base_data();
    data.in_token_data = vec![base_input(), base_input()];
    data.compressions = Some(vec![Compression::decompress(100, 0, 0)]);
    assert!(!check(&data));
}

#[test]
fn test_is_idempotent_ata_decompress_compress_mode() {
    let mut data = base_data();
    data.in_token_data = vec![base_input()];
    data.compressions = Some(vec![Compression::compress(100, 0, 0, 0)]);
    data.in_tlv = Some(vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0,
            is_ata: true,
            bump: 0,
            owner_index: 0,
        },
    )]]);
    assert!(!check(&data));
}

#[test]
fn test_is_idempotent_ata_decompress_not_ata() {
    let mut data = base_data();
    data.in_token_data = vec![base_input()];
    data.compressions = Some(vec![Compression::decompress(100, 0, 0)]);
    data.in_tlv = Some(vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0,
            is_ata: false,
            bump: 0,
            owner_index: 0,
        },
    )]]);
    assert!(!check(&data));
}

#[test]
fn test_is_idempotent_ata_decompress_random_always_false() {
    let mut rng = StdRng::seed_from_u64(42);
    for _ in 0..1000 {
        let mut data = base_data();
        let num_inputs = rng.gen_range(0..5);
        data.in_token_data = (0..num_inputs).map(|_| base_input()).collect();

        // Random compressions -- never Decompress mode so result is always false.
        if rng.gen_bool(0.5) {
            let num_compressions = rng.gen_range(0..4);
            data.compressions = Some(
                (0..num_compressions)
                    .map(|_| {
                        if rng.gen_bool(0.5) {
                            Compression::compress(rng.gen(), 0, 0, 0)
                        } else {
                            Compression {
                                mode: CompressionMode::CompressAndClose,
                                amount: rng.gen(),
                                mint: 0,
                                source_or_recipient: 0,
                                authority: 0,
                                pool_account_index: 0,
                                pool_index: 0,
                                bump: 0,
                                decimals: 0,
                            }
                        }
                    })
                    .collect(),
            );
        }

        assert!(
            !check(&data),
            "Expected false for random input with {num_inputs} inputs"
        );
    }
}

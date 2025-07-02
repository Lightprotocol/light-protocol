import * as borsh from '@coral-xyz/borsh';
import { ValidityProof } from '../state/types';
import {
    PackedStateTreeInfo,
    CompressedAccountMeta,
} from '../state/compressed-account';
import {
    CompressionConfigIxData,
    UpdateCompressionConfigData,
    GenericCompressAccountInstruction,
    CompressedAccountData,
    DecompressMultipleAccountsIdempotentData,
} from './types';

/**
 * Borsh schema for initializeCompressionConfig instruction data
 * Note: This is also available from '@lightprotocol/stateless.js' main exports
 */
export const InitializeCompressionConfigSchema: borsh.Layout<CompressionConfigIxData> =
    borsh.struct([
        borsh.u32('compressionDelay'),
        borsh.publicKey('rentRecipient'),
        borsh.vec(borsh.publicKey(), 'addressSpace'),
        borsh.option(borsh.u8(), 'configBump'),
    ]);

/**
 * Borsh schema for updateCompressionConfig instruction data
 */
export const UpdateCompressionConfigSchema: borsh.Layout<UpdateCompressionConfigData> =
    borsh.struct([
        borsh.option(borsh.u32(), 'newCompressionDelay'),
        borsh.option(borsh.publicKey(), 'newRentRecipient'),
        borsh.option(borsh.vec(borsh.publicKey()), 'newAddressSpace'),
        borsh.option(borsh.publicKey(), 'newUpdateAuthority'),
    ]);

/**
 * Borsh schema for ValidityProof
 */
export const ValidityProofSchema: borsh.Layout<ValidityProof> = borsh.struct([
    borsh.array(borsh.u8(), 32, 'a'),
    borsh.array(borsh.u8(), 64, 'b'),
    borsh.array(borsh.u8(), 32, 'c'),
]);

/**
 * Borsh schema for PackedStateTreeInfo
 */
export const PackedStateTreeInfoSchema: borsh.Layout<PackedStateTreeInfo> =
    borsh.struct([
        borsh.u16('rootIndex'),
        borsh.bool('proveByIndex'),
        borsh.u8('merkleTreePubkeyIndex'),
        borsh.u8('queuePubkeyIndex'),
        borsh.u32('leafIndex'),
    ]);

/**
 * Borsh schema for CompressedAccountMeta
 */
export const CompressedAccountMetaSchema: borsh.Layout<CompressedAccountMeta> =
    borsh.struct([
        PackedStateTreeInfoSchema.replicate('treeInfo'),
        borsh.option(borsh.array(borsh.u8(), 32), 'address'),
        borsh.option(borsh.u64(), 'lamports'),
        borsh.u8('outputStateTreeIndex'),
    ]);

/**
 * Borsh schema for GenericCompressAccountInstruction
 */
export const GenericCompressAccountInstructionSchema: borsh.Layout<GenericCompressAccountInstruction> =
    borsh.struct([
        ValidityProofSchema.replicate('proof'),
        CompressedAccountMetaSchema.replicate('compressedAccountMeta'),
    ]);

/**
 * Helper function to create borsh schema for CompressedAccountData
 * This is generic to work with any data type T
 */
export function createCompressedAccountDataSchema<T>(
    dataSchema: borsh.Layout<T>,
): borsh.Layout<CompressedAccountData<T>> {
    return borsh.struct([
        CompressedAccountMetaSchema.replicate('meta'),
        dataSchema.replicate('data'),
        borsh.vec(borsh.vec(borsh.u8()), 'seeds'),
    ]);
}

/**
 * Helper function to create borsh schema for DecompressMultipleAccountsIdempotentData
 * This is generic to work with any data type T
 */
export function createDecompressMultipleAccountsIdempotentDataSchema<T>(
    dataSchema: borsh.Layout<T>,
): borsh.Layout<DecompressMultipleAccountsIdempotentData<T>> {
    return borsh.struct([
        ValidityProofSchema.replicate('proof'),
        borsh.vec(
            createCompressedAccountDataSchema(dataSchema),
            'compressedAccounts',
        ),
        borsh.vec(borsh.u8(), 'bumps'),
        borsh.u8('systemAccountsOffset'),
    ]);
}

/**
 * Serialize instruction data with custom discriminator
 */
export function serializeInstructionData<T>(
    schema: borsh.Layout<T>,
    data: T,
    discriminator: Uint8Array | number[],
): Buffer {
    const buffer = Buffer.alloc(2000);
    const len = schema.encode(data, buffer);
    const serializedData = Buffer.from(new Uint8Array(buffer.slice(0, len)));

    return Buffer.concat([Buffer.from(discriminator), serializedData]);
}

/**
 * Serialize instruction data for initializeCompressionConfig using Borsh
 */
export function serializeInitializeCompressionConfigData(
    compressionDelay: number,
    rentRecipient: import('@solana/web3.js').PublicKey,
    addressSpace: import('@solana/web3.js').PublicKey[],
    configBump: number | null,
): Buffer {
    const discriminator = Buffer.from([133, 228, 12, 169, 56, 76, 222, 61]);

    const instructionData: CompressionConfigIxData = {
        compressionDelay,
        rentRecipient,
        addressSpace,
        configBump,
    };

    const buffer = Buffer.alloc(1000);
    const len = InitializeCompressionConfigSchema.encode(
        instructionData,
        buffer,
    );
    const dataBuffer = Buffer.from(new Uint8Array(buffer.slice(0, len)));

    return Buffer.concat([
        new Uint8Array(discriminator),
        new Uint8Array(dataBuffer),
    ]);
}

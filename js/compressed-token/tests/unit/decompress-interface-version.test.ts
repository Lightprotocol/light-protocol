import { describe, expect, it } from 'vitest';
import { Keypair } from '@solana/web3.js';
import {
    array,
    bool,
    option,
    struct,
    u16,
    u32,
    u64,
    u8,
    vec,
} from '@coral-xyz/borsh';
import {
    LIGHT_TOKEN_PROGRAM_ID,
    TreeType,
    bn,
} from '@lightprotocol/stateless.js';
import { createDecompressInterfaceInstruction } from '../../src/v3/instructions/create-decompress-interface-instruction';
import { TokenDataVersion } from '../../src/constants';

const CompressionLayout = struct([
    u8('mode'),
    u64('amount'),
    u8('mint'),
    u8('sourceOrRecipient'),
    u8('authority'),
    u8('poolAccountIndex'),
    u8('poolIndex'),
    u8('bump'),
    u8('decimals'),
]);

const PackedMerkleContextLayout = struct([
    u8('merkleTreePubkeyIndex'),
    u8('queuePubkeyIndex'),
    u32('leafIndex'),
    bool('proveByIndex'),
]);

const MultiInputTokenDataWithContextLayout = struct([
    u8('owner'),
    u64('amount'),
    bool('hasDelegate'),
    u8('delegate'),
    u8('mint'),
    u8('version'),
    PackedMerkleContextLayout.replicate('merkleContext'),
    u16('rootIndex'),
]);

const MultiTokenTransferOutputDataLayout = struct([
    u8('owner'),
    u64('amount'),
    bool('hasDelegate'),
    u8('delegate'),
    u8('mint'),
    u8('version'),
]);

const CompressedCpiContextLayout = struct([
    bool('setContext'),
    bool('firstSetContext'),
    u8('cpiContextAccountIndex'),
]);

const CompressedProofLayout = struct([
    array(u8(), 32, 'a'),
    array(u8(), 64, 'b'),
    array(u8(), 32, 'c'),
]);

const Transfer2InstructionDataBaseLayout = struct([
    bool('withTransactionHash'),
    bool('withLamportsChangeAccountMerkleTreeIndex'),
    u8('lamportsChangeAccountMerkleTreeIndex'),
    u8('lamportsChangeAccountOwnerIndex'),
    u8('outputQueue'),
    u16('maxTopUp'),
    option(CompressedCpiContextLayout, 'cpiContext'),
    option(vec(CompressionLayout), 'compressions'),
    option(CompressedProofLayout, 'proof'),
    vec(MultiInputTokenDataWithContextLayout, 'inTokenData'),
    vec(MultiTokenTransferOutputDataLayout, 'outTokenData'),
    option(vec(u64()), 'inLamports'),
    option(vec(u64()), 'outLamports'),
]);

function decodeTransfer2Base(ixData: Buffer): any {
    // Strip Transfer2 discriminator byte; decode only base payload.
    return Transfer2InstructionDataBaseLayout.decode(ixData.subarray(1));
}

function buildCompressedAccount(discriminator?: number[]) {
    const mint = Keypair.generate().publicKey;
    const owner = Keypair.generate().publicKey;
    return {
        parsed: {
            mint,
            owner,
            amount: bn('100'),
            delegate: null,
            state: 1,
            tlv: null,
        },
        compressedAccount: {
            hash: new Uint8Array(32),
            treeInfo: {
                tree: Keypair.generate().publicKey,
                queue: Keypair.generate().publicKey,
                treeType: TreeType.StateV2,
            },
            leafIndex: 0,
            proveByIndex: false,
            owner: LIGHT_TOKEN_PROGRAM_ID,
            lamports: bn(0),
            address: null,
            data: discriminator
                ? {
                      discriminator,
                      data: Buffer.alloc(0),
                      dataHash: new Array(32).fill(0),
                  }
                : undefined,
            readOnly: false,
        },
    };
}

describe('createDecompressInterfaceInstruction version mapping', () => {
    const payer = Keypair.generate().publicKey;
    const destination = Keypair.generate().publicKey;
    const proof = { compressedProof: null, rootIndices: [0] };

    it('maps V1 discriminator to TokenDataVersion.V1 for input and change output', () => {
        const acc = buildCompressedAccount([2, 0, 0, 0, 0, 0, 0, 0]);
        const ix = createDecompressInterfaceInstruction(
            payer,
            [acc as any],
            destination,
            50n,
            proof as any,
            undefined,
            9,
        );
        const decoded = decodeTransfer2Base(ix.data);
        expect(decoded.inTokenData[0].version).toBe(TokenDataVersion.V1);
        expect(decoded.outTokenData[0].version).toBe(TokenDataVersion.V1);
    });

    it('maps version byte 3 to TokenDataVersion.V2', () => {
        const acc = buildCompressedAccount([0, 0, 0, 0, 0, 0, 0, 3]);
        const ix = createDecompressInterfaceInstruction(
            payer,
            [acc as any],
            destination,
            100n,
            proof as any,
            undefined,
            9,
        );
        const decoded = decodeTransfer2Base(ix.data);
        expect(decoded.inTokenData[0].version).toBe(TokenDataVersion.V2);
    });

    it('maps version byte 4 to TokenDataVersion.ShaFlat', () => {
        const acc = buildCompressedAccount([0, 0, 0, 0, 0, 0, 0, 4]);
        const ix = createDecompressInterfaceInstruction(
            payer,
            [acc as any],
            destination,
            100n,
            proof as any,
            undefined,
            9,
        );
        const decoded = decodeTransfer2Base(ix.data);
        expect(decoded.inTokenData[0].version).toBe(TokenDataVersion.ShaFlat);
    });

    it('defaults unknown or missing discriminator to TokenDataVersion.ShaFlat', () => {
        const unknown = buildCompressedAccount([0, 0, 0, 0, 0, 0, 0, 99]);
        const missing = buildCompressedAccount(undefined);

        const unknownIx = createDecompressInterfaceInstruction(
            payer,
            [unknown as any],
            destination,
            100n,
            proof as any,
            undefined,
            9,
        );
        const missingIx = createDecompressInterfaceInstruction(
            payer,
            [missing as any],
            destination,
            100n,
            proof as any,
            undefined,
            9,
        );

        expect(decodeTransfer2Base(unknownIx.data).inTokenData[0].version).toBe(
            TokenDataVersion.ShaFlat,
        );
        expect(decodeTransfer2Base(missingIx.data).inTokenData[0].version).toBe(
            TokenDataVersion.ShaFlat,
        );
    });
});

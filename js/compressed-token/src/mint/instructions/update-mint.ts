import {
    PublicKey,
    SystemProgram,
    TransactionInstruction,
} from '@solana/web3.js';
import { Buffer } from 'buffer';
import {
    ValidityProofWithContext,
    CTOKEN_PROGRAM_ID,
    LightSystemProgram,
    defaultStaticAccountsStruct,
    deriveAddressV2,
    getDefaultAddressTreeInfo,
    MerkleContext,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../../program';
import { findMintAddress } from '../../compressible/derivation';
import {
    struct,
    option,
    vec,
    u8,
    publicKey,
    array,
    u16,
    u32,
    vecU8,
} from '@coral-xyz/borsh';

const MINT_ACTION_DISCRIMINATOR = Buffer.from([106]);

const CompressedProofLayout = struct([
    array(u8(), 32, 'a'),
    array(u8(), 64, 'b'),
    array(u8(), 32, 'c'),
]);

const CompressedMintMetadataLayout = struct([
    u8('version'),
    u8('splMintInitialized'),
    publicKey('splMint'),
]);

const TokenMetadataInstructionDataLayout = struct([
    option(publicKey(), 'updateAuthority'),
    vecU8('name'),
    vecU8('symbol'),
    vecU8('uri'),
    option(
        vec(struct([vecU8('key'), vecU8('value')]), 'additionalMetadata'),
        'additionalMetadata',
    ),
]);

const UpdateAuthorityLayout = struct([option(publicKey(), 'newAuthority')]);

interface EncodeUpdateMintInstructionParams {
    mintSigner: PublicKey;
    currentAuthority: PublicKey;
    newAuthority: PublicKey | null;
    actionType: 'mintAuthority' | 'freezeAuthority';
    addressTree: PublicKey;
    outputQueue: PublicKey;
    leafIndex: number;
    proveByIndex: boolean;
    rootIndex: number;
    proof: ValidityProof | null;
    mintData: {
        supply: bigint;
        decimals: number;
        mintAuthority: PublicKey | null;
        freezeAuthority: PublicKey | null;
        splMint: PublicKey;
        splMintInitialized: boolean;
        version: number;
        metadata?: {
            updateAuthority: PublicKey | null;
            name: string;
            symbol: string;
            uri: string;
        };
    };
}

interface ValidityProof {
    a: number[];
    b: number[];
    c: number[];
}

function encodeUpdateMintInstructionData(
    params: EncodeUpdateMintInstructionParams,
): Buffer {
    const buffer = Buffer.alloc(4000);
    let offset = 0;

    buffer[offset++] = 0;

    buffer[offset++] = 0;

    buffer.writeUInt32LE(params.leafIndex, offset);
    offset += 4;

    buffer[offset++] = params.proveByIndex ? 1 : 0;

    buffer.writeUInt16LE(params.rootIndex, offset);
    offset += 2;

    const compressedAddress = deriveAddressV2(
        params.mintData.splMint.toBytes(),
        params.addressTree.toBytes(),
        CTOKEN_PROGRAM_ID.toBytes(),
    );
    buffer.set(Buffer.from(compressedAddress), offset);
    offset += 32;

    const supplyBytes = Buffer.alloc(8);
    supplyBytes.writeBigUInt64LE(params.mintData.supply);
    buffer.set(supplyBytes, offset);
    offset += 8;
    buffer[offset++] = params.mintData.decimals;
    const metaBuf = Buffer.alloc(64);
    const metaLen = CompressedMintMetadataLayout.encode(
        {
            version: params.mintData.version,
            splMintInitialized: params.mintData.splMintInitialized ? 1 : 0,
            splMint: params.mintData.splMint,
        },
        metaBuf,
    );
    buffer.set(metaBuf.subarray(0, metaLen), offset);
    offset += metaLen;
    if (params.mintData.mintAuthority) {
        buffer[offset++] = 1;
        buffer.set(params.mintData.mintAuthority.toBytes(), offset);
        offset += 32;
    } else {
        buffer[offset++] = 0;
    }
    if (params.mintData.freezeAuthority) {
        buffer[offset++] = 1;
        buffer.set(params.mintData.freezeAuthority.toBytes(), offset);
        offset += 32;
    } else {
        buffer[offset++] = 0;
    }

    if (params.mintData.metadata) {
        buffer[offset++] = 1;
        buffer.writeUInt32LE(1, offset);
        offset += 4;
        buffer[offset++] = 19;
        const mdBuf = Buffer.alloc(2000);
        const mdLen = TokenMetadataInstructionDataLayout.encode(
            {
                updateAuthority:
                    params.mintData.metadata.updateAuthority ?? null,
                name: Buffer.from(params.mintData.metadata.name),
                symbol: Buffer.from(params.mintData.metadata.symbol),
                uri: Buffer.from(params.mintData.metadata.uri),
                additionalMetadata: null,
            },
            mdBuf,
        );
        buffer.set(mdBuf.subarray(0, mdLen), offset);
        offset += mdLen;
    } else {
        buffer[offset++] = 0;
    }

    buffer[offset++] = 0;
    buffer[offset++] = 0;

    buffer.writeUInt32LE(1, offset);
    offset += 4;

    if (params.actionType === 'mintAuthority') {
        buffer[offset++] = 1;
    } else {
        buffer[offset++] = 2;
    }

    const authBuf = Buffer.alloc(64);
    const authLen = UpdateAuthorityLayout.encode(
        { newAuthority: params.newAuthority },
        authBuf,
    );
    buffer.set(authBuf.subarray(0, authLen), offset);
    offset += authLen;

    if (params.proof) {
        buffer[offset++] = 1;
        const prBuf = Buffer.alloc(200);
        const prLen = CompressedProofLayout.encode(params.proof as any, prBuf);
        buffer.set(prBuf.subarray(0, prLen), offset);
        offset += prLen;
    } else {
        buffer[offset++] = 0;
    }

    buffer[offset++] = 0;

    return Buffer.concat([
        MINT_ACTION_DISCRIMINATOR,
        buffer.subarray(0, offset),
    ]);
}

export function createUpdateMintAuthorityInstruction(
    mintSigner: PublicKey,
    currentMintAuthority: PublicKey,
    newMintAuthority: PublicKey | null,
    payer: PublicKey,
    validityProof: ValidityProofWithContext,
    merkleContext: MerkleContext,
    mintData: {
        supply: bigint;
        decimals: number;
        mintAuthority: PublicKey | null;
        freezeAuthority: PublicKey | null;
        splMint: PublicKey;
        splMintInitialized: boolean;
        version: number;
        metadata?: {
            updateAuthority: PublicKey | null;
            name: string;
            symbol: string;
            uri: string;
        };
    },
    outputQueue: PublicKey,
): TransactionInstruction {
    const addressTreeInfo = getDefaultAddressTreeInfo();
    const data = encodeUpdateMintInstructionData({
        mintSigner,
        currentAuthority: currentMintAuthority,
        newAuthority: newMintAuthority,
        actionType: 'mintAuthority',
        addressTree: addressTreeInfo.tree,
        outputQueue,
        leafIndex: merkleContext.leafIndex,
        proveByIndex: true,
        rootIndex: validityProof.rootIndices[0],
        proof: validityProof.compressedProof,
        mintData,
    });

    const sys = defaultStaticAccountsStruct();
    const keys = [
        {
            pubkey: LightSystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: currentMintAuthority, isSigner: true, isWritable: false },
        { pubkey: payer, isSigner: true, isWritable: true },
        {
            pubkey: CompressedTokenProgram.deriveCpiAuthorityPda,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: sys.registeredProgramPda,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: sys.accountCompressionAuthority,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: sys.accountCompressionProgram,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        { pubkey: outputQueue, isSigner: false, isWritable: true },
        {
            pubkey: merkleContext.treeInfo.tree,
            isSigner: false,
            isWritable: true,
        },
        {
            pubkey: merkleContext.treeInfo.queue,
            isSigner: false,
            isWritable: true,
        },
    ];

    return new TransactionInstruction({
        programId: CTOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

export function createUpdateFreezeAuthorityInstruction(
    mintSigner: PublicKey,
    currentFreezeAuthority: PublicKey,
    newFreezeAuthority: PublicKey | null,
    payer: PublicKey,
    validityProof: ValidityProofWithContext,
    merkleContext: MerkleContext,
    mintData: {
        supply: bigint;
        decimals: number;
        mintAuthority: PublicKey | null;
        freezeAuthority: PublicKey | null;
        splMint: PublicKey;
        splMintInitialized: boolean;
        version: number;
        metadata?: {
            updateAuthority: PublicKey | null;
            name: string;
            symbol: string;
            uri: string;
        };
    },
    outputQueue: PublicKey,
): TransactionInstruction {
    const addressTreeInfo = getDefaultAddressTreeInfo();
    const data = encodeUpdateMintInstructionData({
        mintSigner,
        currentAuthority: currentFreezeAuthority,
        newAuthority: newFreezeAuthority,
        actionType: 'freezeAuthority',
        addressTree: addressTreeInfo.tree,
        outputQueue,
        leafIndex: merkleContext.leafIndex,
        proveByIndex: true,
        rootIndex: validityProof.rootIndices[0],
        proof: validityProof.compressedProof,
        mintData,
    });

    const sys = defaultStaticAccountsStruct();
    const keys = [
        {
            pubkey: LightSystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: currentFreezeAuthority, isSigner: true, isWritable: false },
        { pubkey: payer, isSigner: true, isWritable: true },
        {
            pubkey: CompressedTokenProgram.deriveCpiAuthorityPda,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: sys.registeredProgramPda,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: sys.accountCompressionAuthority,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: sys.accountCompressionProgram,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        { pubkey: outputQueue, isSigner: false, isWritable: true },
        {
            pubkey: merkleContext.treeInfo.tree,
            isSigner: false,
            isWritable: true,
        },
        {
            pubkey: merkleContext.treeInfo.queue,
            isSigner: false,
            isWritable: true,
        },
    ];

    return new TransactionInstruction({
        programId: CTOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

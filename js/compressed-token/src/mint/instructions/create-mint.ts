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
    TreeInfo,
    AddressTreeInfo,
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
    vecU8,
} from '@coral-xyz/borsh';

const MINT_ACTION_DISCRIMINATOR = Buffer.from([106]);

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

const CompressedMintInstructionDataLayout = struct([
    array(u8(), 8, 'supplyLE'),
    u8('decimals'),
    CompressedMintMetadataLayout.replicate('metadata'),
    option(publicKey(), 'mintAuthority'),
    option(publicKey(), 'freezeAuthority'),
]);

export interface TokenMetadataInstructionData {
    name: string;
    symbol: string;
    uri: string;
    updateAuthority?: PublicKey | null;
    additionalMetadata?: {
        key: string;
        value: string;
    }[];
}

interface EncodeCreateMintInstructionParams {
    mintSigner: PublicKey;
    mintAuthority: PublicKey;
    freezeAuthority: PublicKey | null;
    decimals: number;
    addressTree: PublicKey;
    outputQueue: PublicKey;
    rootIndex: number;
    proof: ValidityProof | null;
    metadata?: TokenMetadataInstructionData;
}

interface ValidityProof {
    a: number[];
    b: number[];
    c: number[];
}

export function createTokenMetadata(
    name: string,
    symbol: string,
    uri: string,
    updateAuthority?: PublicKey | null,
): TokenMetadataInstructionData {
    return {
        name,
        symbol,
        uri,
        updateAuthority: updateAuthority ?? null,
    };
}

function encodeCreateMintInstructionData(
    params: EncodeCreateMintInstructionParams,
): Buffer {
    const buffer = Buffer.alloc(4000);
    let offset = 0;

    buffer[offset++] = 1;

    const [, mintBump] = findMintAddress(params.mintSigner);
    buffer[offset++] = mintBump;

    buffer.writeUInt32LE(0, offset);
    offset += 4;

    buffer[offset++] = 0;

    buffer.writeUInt16LE(params.rootIndex, offset);
    offset += 2;

    const [splMintPda] = findMintAddress(params.mintSigner);
    const compressedAddress = deriveAddressV2(
        splMintPda.toBytes(),
        params.addressTree.toBytes(),
        CTOKEN_PROGRAM_ID.toBytes(),
    );
    buffer.set(Buffer.from(compressedAddress), offset);
    offset += 32;

    buffer.set(Buffer.alloc(8, 0), offset);
    offset += 8;
    buffer[offset++] = params.decimals;
    const metaBuf = Buffer.alloc(64);
    const metaLen = CompressedMintMetadataLayout.encode(
        {
            version: 3,
            splMintInitialized: 0,
            splMint: splMintPda,
        },
        metaBuf,
    );
    buffer.set(metaBuf.subarray(0, metaLen), offset);
    offset += metaLen;
    if (params.mintAuthority) {
        buffer[offset++] = 1;
        buffer.set(params.mintAuthority.toBytes(), offset);
        offset += 32;
    } else {
        buffer[offset++] = 0;
    }
    if (params.freezeAuthority) {
        buffer[offset++] = 1;
        buffer.set(params.freezeAuthority.toBytes(), offset);
        offset += 32;
    } else {
        buffer[offset++] = 0;
    }
    if (params.metadata) {
        buffer[offset++] = 1;
        buffer.writeUInt32LE(1, offset);
        offset += 4;
        buffer[offset++] = 19;
        const mdBuf = Buffer.alloc(2000);
        const mdLen = TokenMetadataInstructionDataLayout.encode(
            {
                updateAuthority: params.metadata.updateAuthority ?? null,
                name: Buffer.from(params.metadata.name),
                symbol: Buffer.from(params.metadata.symbol),
                uri: Buffer.from(params.metadata.uri),
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

    buffer.writeUInt32LE(0, offset);
    offset += 4;

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

export function createMintInstruction(
    mintSigner: PublicKey,
    decimals: number,
    mintAuthority: PublicKey,
    freezeAuthority: PublicKey | null,
    payer: PublicKey,
    validityProof: ValidityProofWithContext,
    metadata: TokenMetadataInstructionData | undefined,
    addressTreeInfo: AddressTreeInfo,
    outputStateTreeInfo: TreeInfo,
): TransactionInstruction {
    const data = encodeCreateMintInstructionData({
        mintSigner,
        mintAuthority,
        freezeAuthority,
        decimals,
        addressTree: addressTreeInfo.tree,
        outputQueue: outputStateTreeInfo.queue,
        rootIndex: validityProof.rootIndices[0],
        proof: validityProof.compressedProof,
        metadata,
    });

    const sys = defaultStaticAccountsStruct();
    const keys = [
        {
            pubkey: LightSystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: mintSigner, isSigner: true, isWritable: false },
        { pubkey: mintAuthority, isSigner: true, isWritable: false },
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
        {
            pubkey: outputStateTreeInfo.queue,
            isSigner: false,
            isWritable: true,
        },
        {
            pubkey: addressTreeInfo.tree,
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

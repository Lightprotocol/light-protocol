import {
    PublicKey,
    SystemProgram,
    TransactionInstruction,
} from '@solana/web3.js';
import { Buffer } from 'buffer';
import BN from 'bn.js';
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
import {
    struct,
    option,
    vec,
    u8,
    publicKey,
    array,
    u16,
    u64,
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

const DecompressedRecipientLayout = struct([u8('accountIndex'), u64('amount')]);

const MintToCTokenActionLayout = struct([
    DecompressedRecipientLayout.replicate('recipient'),
]);

interface EncodeMintToCTokenInstructionParams {
    mintSigner: PublicKey;
    addressTree: PublicKey;
    outputQueue: PublicKey;
    leafIndex: number;
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
    recipientAccount: PublicKey;
    recipientAccountIndex: number;
    amount: number | bigint;
}

interface ValidityProof {
    a: number[];
    b: number[];
    c: number[];
}

function encodeMintToCTokenInstructionData(
    params: EncodeMintToCTokenInstructionParams,
): Buffer {
    const buffer = Buffer.alloc(4000);
    let offset = 0;

    buffer[offset++] = 0;

    buffer[offset++] = 0;

    buffer.writeUInt32LE(params.leafIndex, offset);
    offset += 4;

    buffer[offset++] = 1;

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
        throw new Error(
            'TokenMetadata extension not supported in mintTo instruction',
        );
    } else {
        buffer[offset++] = 0;
    }

    buffer[offset++] = 0;
    buffer[offset++] = 0;

    buffer.writeUInt32LE(1, offset);
    offset += 4;

    buffer[offset++] = 4;

    const actionBuf = Buffer.alloc(200);
    const actionLen = MintToCTokenActionLayout.encode(
        {
            recipient: {
                accountIndex: params.recipientAccountIndex,
                amount: new BN(params.amount.toString()),
            },
        },
        actionBuf,
    );
    buffer.set(actionBuf.subarray(0, actionLen), offset);
    offset += actionLen;

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

export function createMintToInstruction(
    mintSigner: PublicKey,
    authority: PublicKey,
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
    tokensOutQueue: PublicKey,
    recipientAccount: PublicKey,
    amount: number | bigint,
): TransactionInstruction {
    const addressTreeInfo = getDefaultAddressTreeInfo();
    const data = encodeMintToCTokenInstructionData({
        mintSigner,
        addressTree: addressTreeInfo.tree,
        outputQueue,
        leafIndex: merkleContext.leafIndex,
        rootIndex: validityProof.rootIndices[0],
        proof: validityProof.compressedProof,
        mintData,
        recipientAccount,
        recipientAccountIndex: 0,
        amount,
    });

    const sys = defaultStaticAccountsStruct();
    const keys = [
        {
            pubkey: LightSystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: authority, isSigner: true, isWritable: false },
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
        { pubkey: tokensOutQueue, isSigner: false, isWritable: true },
    ];

    keys.push({ pubkey: recipientAccount, isSigner: false, isWritable: true });

    return new TransactionInstruction({
        programId: CTOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

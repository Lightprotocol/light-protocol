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

const UpdateMetadataFieldActionLayout = struct([
    u8('extensionIndex'),
    u8('fieldType'),
    vecU8('key'),
    vecU8('value'),
]);

const UpdateMetadataAuthorityActionLayout = struct([
    u8('extensionIndex'),
    publicKey('newAuthority'),
]);

const RemoveMetadataKeyActionLayout = struct([
    u8('extensionIndex'),
    vecU8('key'),
    u8('idempotent'),
]);

interface EncodeUpdateMetadataInstructionParams {
    mintSigner: PublicKey;
    authority: PublicKey;
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
        metadata: {
            updateAuthority: PublicKey | null;
            name: string;
            symbol: string;
            uri: string;
        };
    };
    action: UpdateMetadataAction;
}

interface ValidityProof {
    a: number[];
    b: number[];
    c: number[];
}

type UpdateMetadataAction =
    | {
          type: 'updateField';
          extensionIndex: number;
          fieldType: number;
          key: string;
          value: string;
      }
    | {
          type: 'updateAuthority';
          extensionIndex: number;
          newAuthority: PublicKey;
      }
    | {
          type: 'removeKey';
          extensionIndex: number;
          key: string;
          idempotent: boolean;
      };

function encodeUpdateMetadataInstructionData(
    params: EncodeUpdateMetadataInstructionParams,
): Buffer {
    const buffer = Buffer.alloc(4000);
    let offset = 0;

    buffer[offset++] = 0;

    const [, mintBump] = findMintAddress(params.mintSigner);
    buffer[offset++] = mintBump;

    buffer.writeUInt32LE(params.leafIndex, offset);
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

    buffer[offset++] = 1;
    buffer.writeUInt32LE(1, offset);
    offset += 4;
    buffer[offset++] = 19;
    const mdBuf = Buffer.alloc(2000);
    const mdLen = TokenMetadataInstructionDataLayout.encode(
        {
            updateAuthority: params.mintData.metadata.updateAuthority ?? null,
            name: Buffer.from(params.mintData.metadata.name),
            symbol: Buffer.from(params.mintData.metadata.symbol),
            uri: Buffer.from(params.mintData.metadata.uri),
            additionalMetadata: null,
        },
        mdBuf,
    );
    buffer.set(mdBuf.subarray(0, mdLen), offset);
    offset += mdLen;

    buffer[offset++] = 0;
    buffer[offset++] = 0;

    buffer.writeUInt32LE(1, offset);
    offset += 4;

    if (params.action.type === 'updateField') {
        buffer[offset++] = 5;
        const actionBuf = Buffer.alloc(2000);
        const actionLen = UpdateMetadataFieldActionLayout.encode(
            {
                extensionIndex: params.action.extensionIndex,
                fieldType: params.action.fieldType,
                key: Buffer.from(params.action.key),
                value: Buffer.from(params.action.value),
            },
            actionBuf,
        );
        buffer.set(actionBuf.subarray(0, actionLen), offset);
        offset += actionLen;
    } else if (params.action.type === 'updateAuthority') {
        buffer[offset++] = 6;
        const actionBuf = Buffer.alloc(64);
        const actionLen = UpdateMetadataAuthorityActionLayout.encode(
            {
                extensionIndex: params.action.extensionIndex,
                newAuthority: params.action.newAuthority,
            },
            actionBuf,
        );
        buffer.set(actionBuf.subarray(0, actionLen), offset);
        offset += actionLen;
    } else {
        buffer[offset++] = 7;
        const actionBuf = Buffer.alloc(2000);
        const actionLen = RemoveMetadataKeyActionLayout.encode(
            {
                extensionIndex: params.action.extensionIndex,
                key: Buffer.from(params.action.key),
                idempotent: params.action.idempotent ? 1 : 0,
            },
            actionBuf,
        );
        buffer.set(actionBuf.subarray(0, actionLen), offset);
        offset += actionLen;
    }

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

function createUpdateMetadataInstruction(
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
        metadata: {
            updateAuthority: PublicKey | null;
            name: string;
            symbol: string;
            uri: string;
        };
    },
    outputQueue: PublicKey,
    action: UpdateMetadataAction,
): TransactionInstruction {
    const addressTreeInfo = getDefaultAddressTreeInfo();
    const data = encodeUpdateMetadataInstructionData({
        mintSigner,
        authority,
        addressTree: addressTreeInfo.tree,
        outputQueue,
        leafIndex: merkleContext.leafIndex,
        rootIndex: validityProof.rootIndices[0],
        proof: validityProof.compressedProof,
        mintData,
        action,
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
    ];

    return new TransactionInstruction({
        programId: CTOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

export function createUpdateMetadataFieldInstruction(
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
        metadata: {
            updateAuthority: PublicKey | null;
            name: string;
            symbol: string;
            uri: string;
        };
    },
    outputQueue: PublicKey,
    fieldType: 'name' | 'symbol' | 'uri' | 'custom',
    value: string,
    customKey?: string,
    extensionIndex: number = 0,
): TransactionInstruction {
    const action: UpdateMetadataAction = {
        type: 'updateField',
        extensionIndex,
        fieldType:
            fieldType === 'name'
                ? 0
                : fieldType === 'symbol'
                  ? 1
                  : fieldType === 'uri'
                    ? 2
                    : 3,
        key: customKey || '',
        value,
    };

    return createUpdateMetadataInstruction(
        mintSigner,
        authority,
        payer,
        validityProof,
        merkleContext,
        mintData,
        outputQueue,
        action,
    );
}

export function createUpdateMetadataAuthorityInstruction(
    mintSigner: PublicKey,
    currentAuthority: PublicKey,
    newAuthority: PublicKey,
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
        metadata: {
            updateAuthority: PublicKey | null;
            name: string;
            symbol: string;
            uri: string;
        };
    },
    outputQueue: PublicKey,
    extensionIndex: number = 0,
): TransactionInstruction {
    const action: UpdateMetadataAction = {
        type: 'updateAuthority',
        extensionIndex,
        newAuthority,
    };

    return createUpdateMetadataInstruction(
        mintSigner,
        currentAuthority,
        payer,
        validityProof,
        merkleContext,
        mintData,
        outputQueue,
        action,
    );
}

export function createRemoveMetadataKeyInstruction(
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
        metadata: {
            updateAuthority: PublicKey | null;
            name: string;
            symbol: string;
            uri: string;
        };
    },
    outputQueue: PublicKey,
    key: string,
    idempotent: boolean = false,
    extensionIndex: number = 0,
): TransactionInstruction {
    const action: UpdateMetadataAction = {
        type: 'removeKey',
        extensionIndex,
        key,
        idempotent,
    };

    return createUpdateMetadataInstruction(
        mintSigner,
        authority,
        payer,
        validityProof,
        merkleContext,
        mintData,
        outputQueue,
        action,
    );
}

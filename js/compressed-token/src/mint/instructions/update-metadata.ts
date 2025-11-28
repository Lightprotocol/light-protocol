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
import { MintInstructionDataWithMetadata } from '../serde';
import {
    encodeMintActionInstructionData,
    MintActionCompressedInstructionData,
    Action,
} from './mint-action-layout';

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

interface EncodeUpdateMetadataInstructionParams {
    mintSigner: PublicKey;
    addressTree: PublicKey;
    leafIndex: number;
    rootIndex: number;
    proof: { a: number[]; b: number[]; c: number[] } | null;
    mintData: MintInstructionDataWithMetadata;
    action: UpdateMetadataAction;
}

function convertActionToBorsh(action: UpdateMetadataAction): Action {
    if (action.type === 'updateField') {
        return {
            updateMetadataField: {
                extensionIndex: action.extensionIndex,
                fieldType: action.fieldType,
                key: Buffer.from(action.key),
                value: Buffer.from(action.value),
            },
        };
    } else if (action.type === 'updateAuthority') {
        return {
            updateMetadataAuthority: {
                extensionIndex: action.extensionIndex,
                newAuthority: action.newAuthority,
            },
        };
    } else {
        return {
            removeMetadataKey: {
                extensionIndex: action.extensionIndex,
                key: Buffer.from(action.key),
                idempotent: action.idempotent ? 1 : 0,
            },
        };
    }
}

function encodeUpdateMetadataInstructionData(
    params: EncodeUpdateMetadataInstructionParams,
): Buffer {
    const [splMintPda] = findMintAddress(params.mintSigner);
    const compressedAddress = deriveAddressV2(
        splMintPda.toBytes(),
        params.addressTree,
        CTOKEN_PROGRAM_ID,
    );

    const instructionData: MintActionCompressedInstructionData = {
        leafIndex: params.leafIndex,
        proveByIndex: params.proof === null,
        rootIndex: params.rootIndex,
        compressedAddress: Array.from(compressedAddress.toBytes()),
        tokenPoolBump: 0,
        tokenPoolIndex: 0,
        createMint: null,
        actions: [convertActionToBorsh(params.action)],
        proof: params.proof,
        cpiContext: null,
        mint: {
            supply: params.mintData.supply,
            decimals: params.mintData.decimals,
            metadata: {
                version: params.mintData.version,
                splMintInitialized: params.mintData.splMintInitialized,
                mint: params.mintData.splMint,
            },
            mintAuthority: params.mintData.mintAuthority,
            freezeAuthority: params.mintData.freezeAuthority,
            extensions: [
                {
                    tokenMetadata: {
                        updateAuthority:
                            params.mintData.metadata.updateAuthority ?? null,
                        name: Buffer.from(params.mintData.metadata.name),
                        symbol: Buffer.from(params.mintData.metadata.symbol),
                        uri: Buffer.from(params.mintData.metadata.uri),
                        additionalMetadata: null,
                    },
                },
            ],
        },
    };

    return encodeMintActionInstructionData(instructionData);
}

interface CreateUpdateMetadataInstructionParams {
    mintSigner: PublicKey;
    authority: PublicKey;
    payer: PublicKey;
    validityProof: ValidityProofWithContext;
    merkleContext: MerkleContext;
    mintData: MintInstructionDataWithMetadata;
    outputQueue: PublicKey;
    action: UpdateMetadataAction;
}

function createUpdateMetadataInstruction({
    mintSigner,
    authority,
    payer,
    validityProof,
    merkleContext,
    mintData,
    outputQueue,
    action,
}: CreateUpdateMetadataInstructionParams): TransactionInstruction {
    const addressTreeInfo = getDefaultAddressTreeInfo();
    const data = encodeUpdateMetadataInstructionData({
        mintSigner,
        addressTree: addressTreeInfo.tree,
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

export interface CreateUpdateMetadataFieldInstructionParams {
    mintSigner: PublicKey;
    authority: PublicKey;
    payer: PublicKey;
    validityProof: ValidityProofWithContext;
    merkleContext: MerkleContext;
    mintData: MintInstructionDataWithMetadata;
    outputQueue: PublicKey;
    fieldType: 'name' | 'symbol' | 'uri' | 'custom';
    value: string;
    customKey?: string;
    extensionIndex?: number;
}

/**
 * Create instruction for updating a compressed mint's metadata field.
 *
 * @param mintSigner     Mint signer public key.
 * @param authority      Metadata update authority public key.
 * @param payer          Fee payer public key.
 * @param validityProof  Validity proof for the compressed mint.
 * @param merkleContext  Merkle context of the compressed mint.
 * @param mintData       Mint instruction data with metadata.
 * @param outputQueue    Output queue for state changes.
 * @param fieldType      Field to update: 'name', 'symbol', 'uri', or 'custom'.
 * @param value          New value for the field.
 * @param customKey      Custom key name (required if fieldType is 'custom').
 * @param extensionIndex Extension index (default: 0).
 */
export function createUpdateMetadataFieldInstruction({
    mintSigner,
    authority,
    payer,
    validityProof,
    merkleContext,
    mintData,
    outputQueue,
    fieldType,
    value,
    customKey,
    extensionIndex = 0,
}: CreateUpdateMetadataFieldInstructionParams): TransactionInstruction {
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

    return createUpdateMetadataInstruction({
        mintSigner,
        authority,
        payer,
        validityProof,
        merkleContext,
        mintData,
        outputQueue,
        action,
    });
}

export interface CreateUpdateMetadataAuthorityInstructionParams {
    mintSigner: PublicKey;
    currentAuthority: PublicKey;
    newAuthority: PublicKey;
    payer: PublicKey;
    validityProof: ValidityProofWithContext;
    merkleContext: MerkleContext;
    mintData: MintInstructionDataWithMetadata;
    outputQueue: PublicKey;
    extensionIndex?: number;
}

/**
 * Create instruction for updating a compressed mint's metadata authority.
 *
 * @param mintSigner       Mint signer public key.
 * @param currentAuthority Current metadata update authority public key.
 * @param newAuthority     New metadata update authority public key.
 * @param payer            Fee payer public key.
 * @param validityProof    Validity proof for the compressed mint.
 * @param merkleContext    Merkle context of the compressed mint.
 * @param mintData         Mint instruction data with metadata.
 * @param outputQueue      Output queue for state changes.
 * @param extensionIndex   Extension index (default: 0).
 */
export function createUpdateMetadataAuthorityInstruction({
    mintSigner,
    currentAuthority,
    newAuthority,
    payer,
    validityProof,
    merkleContext,
    mintData,
    outputQueue,
    extensionIndex = 0,
}: CreateUpdateMetadataAuthorityInstructionParams): TransactionInstruction {
    const action: UpdateMetadataAction = {
        type: 'updateAuthority',
        extensionIndex,
        newAuthority,
    };

    return createUpdateMetadataInstruction({
        mintSigner,
        authority: currentAuthority,
        payer,
        validityProof,
        merkleContext,
        mintData,
        outputQueue,
        action,
    });
}

export interface CreateRemoveMetadataKeyInstructionParams {
    mintSigner: PublicKey;
    authority: PublicKey;
    payer: PublicKey;
    validityProof: ValidityProofWithContext;
    merkleContext: MerkleContext;
    mintData: MintInstructionDataWithMetadata;
    outputQueue: PublicKey;
    key: string;
    idempotent?: boolean;
    extensionIndex?: number;
}

/**
 * Create instruction for removing a metadata key from a compressed mint.
 *
 * @param mintSigner     Mint signer public key.
 * @param authority      Metadata update authority public key.
 * @param payer          Fee payer public key.
 * @param validityProof  Validity proof for the compressed mint.
 * @param merkleContext  Merkle context of the compressed mint.
 * @param mintData       Mint instruction data with metadata.
 * @param outputQueue    Output queue for state changes.
 * @param key            Metadata key to remove.
 * @param idempotent     If true, don't error if key doesn't exist (default: false).
 * @param extensionIndex Extension index (default: 0).
 */
export function createRemoveMetadataKeyInstruction({
    mintSigner,
    authority,
    payer,
    validityProof,
    merkleContext,
    mintData,
    outputQueue,
    key,
    idempotent = false,
    extensionIndex = 0,
}: CreateRemoveMetadataKeyInstructionParams): TransactionInstruction {
    const action: UpdateMetadataAction = {
        type: 'removeKey',
        extensionIndex,
        key,
        idempotent,
    };

    return createUpdateMetadataInstruction({
        mintSigner,
        authority,
        payer,
        validityProof,
        merkleContext,
        mintData,
        outputQueue,
        action,
    });
}

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

function createUpdateMetadataInstruction(
    mintSigner: PublicKey,
    authority: PublicKey,
    payer: PublicKey,
    validityProof: ValidityProofWithContext,
    merkleContext: MerkleContext,
    mintData: MintInstructionDataWithMetadata,
    outputQueue: PublicKey,
    action: UpdateMetadataAction,
): TransactionInstruction {
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

export function createUpdateMetadataFieldInstruction(
    mintSigner: PublicKey,
    authority: PublicKey,
    payer: PublicKey,
    validityProof: ValidityProofWithContext,
    merkleContext: MerkleContext,
    mintData: MintInstructionDataWithMetadata,
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
    mintData: MintInstructionDataWithMetadata,
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
    mintData: MintInstructionDataWithMetadata,
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

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
    getOutputQueue,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../../program';
import { MintInterface } from '../get-mint-interface';
import {
    encodeMintActionInstructionData,
    MintActionCompressedInstructionData,
    Action,
} from '../layout/layout-mint-action';

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
    splMint: PublicKey;
    addressTree: PublicKey;
    leafIndex: number;
    rootIndex: number;
    proof: { a: number[]; b: number[]; c: number[] } | null;
    mintInterface: MintInterface;
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
    const compressedAddress = deriveAddressV2(
        params.splMint.toBytes(),
        params.addressTree,
        CTOKEN_PROGRAM_ID,
    );

    const mintInterface = params.mintInterface;

    if (!mintInterface.tokenMetadata) {
        throw new Error(
            'MintInterface must have tokenMetadata for metadata operations',
        );
    }

    const instructionData: MintActionCompressedInstructionData = {
        leafIndex: params.leafIndex,
        proveByIndex: params.proof === null,
        rootIndex: params.rootIndex,
        compressedAddress: Array.from(compressedAddress.toBytes()),
        tokenPoolBump: 0,
        tokenPoolIndex: 0,
        maxTopUp: 0,
        createMint: null,
        actions: [convertActionToBorsh(params.action)],
        proof: params.proof,
        cpiContext: null,
        mint: {
            supply: mintInterface.mint.supply,
            decimals: mintInterface.mint.decimals,
            metadata: {
                version: mintInterface.mintContext!.version,
                splMintInitialized:
                    mintInterface.mintContext!.splMintInitialized,
                mint: mintInterface.mintContext!.splMint,
            },
            mintAuthority: mintInterface.mint.mintAuthority,
            freezeAuthority: mintInterface.mint.freezeAuthority,
            extensions: [
                {
                    tokenMetadata: {
                        updateAuthority:
                            mintInterface.tokenMetadata.updateAuthority ?? null,
                        name: Buffer.from(mintInterface.tokenMetadata.name),
                        symbol: Buffer.from(mintInterface.tokenMetadata.symbol),
                        uri: Buffer.from(mintInterface.tokenMetadata.uri),
                        additionalMetadata: null,
                    },
                },
            ],
        },
    };

    return encodeMintActionInstructionData(instructionData);
}

function createUpdateMetadataInstruction(
    mintInterface: MintInterface,
    authority: PublicKey,
    payer: PublicKey,
    validityProof: ValidityProofWithContext,
    action: UpdateMetadataAction,
): TransactionInstruction {
    if (!mintInterface.merkleContext) {
        throw new Error(
            'MintInterface must have merkleContext for compressed mint operations',
        );
    }
    if (!mintInterface.mintContext) {
        throw new Error(
            'MintInterface must have mintContext for compressed mint operations',
        );
    }
    if (!mintInterface.tokenMetadata) {
        throw new Error(
            'MintInterface must have tokenMetadata for metadata operations',
        );
    }

    const merkleContext = mintInterface.merkleContext;
    const outputQueue = getOutputQueue(merkleContext);

    const addressTreeInfo = getDefaultAddressTreeInfo();
    const data = encodeUpdateMetadataInstructionData({
        splMint: mintInterface.mintContext.splMint,
        addressTree: addressTreeInfo.tree,
        leafIndex: merkleContext.leafIndex,
        rootIndex: validityProof.rootIndices[0],
        proof: validityProof.compressedProof,
        mintInterface,
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

/**
 * Create instruction for updating a compressed mint's metadata field.
 *
 * Output queue is automatically derived from mintInterface.merkleContext.treeInfo
 * (preferring nextTreeInfo.queue if available for rollover support).
 *
 * @param mintInterface  MintInterface from getMintInterface() - must have merkleContext and tokenMetadata
 * @param authority      Metadata update authority public key (must sign)
 * @param payer          Fee payer public key
 * @param validityProof  Validity proof for the compressed mint
 * @param fieldType      Field to update: 'name', 'symbol', 'uri', or 'custom'
 * @param value          New value for the field
 * @param customKey      Custom key name (required if fieldType is 'custom')
 * @param extensionIndex Extension index (default: 0)
 */
export function createUpdateMetadataFieldInstruction(
    mintInterface: MintInterface,
    authority: PublicKey,
    payer: PublicKey,
    validityProof: ValidityProofWithContext,
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
        mintInterface,
        authority,
        payer,
        validityProof,
        action,
    );
}

/**
 * Create instruction for updating a compressed mint's metadata authority.
 *
 * Output queue is automatically derived from mintInterface.merkleContext.treeInfo
 * (preferring nextTreeInfo.queue if available for rollover support).
 *
 * @param mintInterface    MintInterface from getMintInterface() - must have merkleContext and tokenMetadata
 * @param currentAuthority Current metadata update authority public key (must sign)
 * @param newAuthority     New metadata update authority public key
 * @param payer            Fee payer public key
 * @param validityProof    Validity proof for the compressed mint
 * @param extensionIndex   Extension index (default: 0)
 */
export function createUpdateMetadataAuthorityInstruction(
    mintInterface: MintInterface,
    currentAuthority: PublicKey,
    newAuthority: PublicKey,
    payer: PublicKey,
    validityProof: ValidityProofWithContext,
    extensionIndex: number = 0,
): TransactionInstruction {
    const action: UpdateMetadataAction = {
        type: 'updateAuthority',
        extensionIndex,
        newAuthority,
    };

    return createUpdateMetadataInstruction(
        mintInterface,
        currentAuthority,
        payer,
        validityProof,
        action,
    );
}

/**
 * Create instruction for removing a metadata key from a compressed mint.
 *
 * Output queue is automatically derived from mintInterface.merkleContext.treeInfo
 * (preferring nextTreeInfo.queue if available for rollover support).
 *
 * @param mintInterface  MintInterface from getMintInterface() - must have merkleContext and tokenMetadata
 * @param authority      Metadata update authority public key (must sign)
 * @param payer          Fee payer public key
 * @param validityProof  Validity proof for the compressed mint
 * @param key            Metadata key to remove
 * @param idempotent     If true, don't error if key doesn't exist (default: false)
 * @param extensionIndex Extension index (default: 0)
 */
export function createRemoveMetadataKeyInstruction(
    mintInterface: MintInterface,
    authority: PublicKey,
    payer: PublicKey,
    validityProof: ValidityProofWithContext,
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
        mintInterface,
        authority,
        payer,
        validityProof,
        action,
    );
}

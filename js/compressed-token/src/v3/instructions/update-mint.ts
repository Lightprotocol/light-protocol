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
    ExtensionInstructionData,
} from '../layout/layout-mint-action';

interface EncodeUpdateMintInstructionParams {
    splMint: PublicKey;
    addressTree: PublicKey;
    leafIndex: number;
    proveByIndex: boolean;
    rootIndex: number;
    proof: { a: number[]; b: number[]; c: number[] } | null;
    mintInterface: MintInterface;
    newAuthority: PublicKey | null;
    actionType: 'mintAuthority' | 'freezeAuthority';
}

function encodeUpdateMintInstructionData(
    params: EncodeUpdateMintInstructionParams,
): Buffer {
    const compressedAddress = deriveAddressV2(
        params.splMint.toBytes(),
        params.addressTree,
        CTOKEN_PROGRAM_ID,
    );

    // Build action
    const action: Action =
        params.actionType === 'mintAuthority'
            ? { updateMintAuthority: { newAuthority: params.newAuthority } }
            : { updateFreezeAuthority: { newAuthority: params.newAuthority } };

    // Build extensions if metadata present
    let extensions: ExtensionInstructionData[] | null = null;
    if (params.mintInterface.tokenMetadata) {
        extensions = [
            {
                tokenMetadata: {
                    updateAuthority:
                        params.mintInterface.tokenMetadata.updateAuthority ??
                        null,
                    name: Buffer.from(params.mintInterface.tokenMetadata.name),
                    symbol: Buffer.from(
                        params.mintInterface.tokenMetadata.symbol,
                    ),
                    uri: Buffer.from(params.mintInterface.tokenMetadata.uri),
                    additionalMetadata: null,
                },
            },
        ];
    }

    const instructionData: MintActionCompressedInstructionData = {
        leafIndex: params.leafIndex,
        proveByIndex: params.proveByIndex,
        rootIndex: params.rootIndex,
        compressedAddress: Array.from(compressedAddress.toBytes()),
        tokenPoolBump: 0,
        tokenPoolIndex: 0,
        maxTopUp: 0,
        createMint: null,
        actions: [action],
        proof: params.proof,
        cpiContext: null,
        mint: {
            supply: params.mintInterface.mint.supply,
            decimals: params.mintInterface.mint.decimals,
            metadata: {
                version: params.mintInterface.mintContext!.version,
                splMintInitialized:
                    params.mintInterface.mintContext!.splMintInitialized,
                mint: params.mintInterface.mintContext!.splMint,
            },
            mintAuthority: params.mintInterface.mint.mintAuthority,
            freezeAuthority: params.mintInterface.mint.freezeAuthority,
            extensions,
        },
    };

    return encodeMintActionInstructionData(instructionData);
}

/**
 * Create instruction for updating a compressed mint's mint authority.
 *
 * @param mintInterface          MintInterface from getMintInterface() - must have merkleContext
 * @param currentMintAuthority   Current mint authority public key (must sign)
 * @param newMintAuthority       New mint authority (or null to revoke)
 * @param payer                  Fee payer public key
 * @param validityProof          Validity proof for the compressed mint
 */
export function createUpdateMintAuthorityInstruction(
    mintInterface: MintInterface,
    currentMintAuthority: PublicKey,
    newMintAuthority: PublicKey | null,
    payer: PublicKey,
    validityProof: ValidityProofWithContext,
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

    const merkleContext = mintInterface.merkleContext;
    const outputQueue = getOutputQueue(merkleContext);

    const addressTreeInfo = getDefaultAddressTreeInfo();
    const data = encodeUpdateMintInstructionData({
        splMint: mintInterface.mintContext.splMint,
        addressTree: addressTreeInfo.tree,
        leafIndex: merkleContext.leafIndex,
        proveByIndex: true,
        rootIndex: validityProof.rootIndices[0],
        proof: validityProof.compressedProof,
        mintInterface,
        newAuthority: newMintAuthority,
        actionType: 'mintAuthority',
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

/**
 * Create instruction for updating a compressed mint's freeze authority.
 *
 * Output queue is automatically derived from mintInterface.merkleContext.treeInfo
 * (preferring nextTreeInfo.queue if available for rollover support).
 *
 * @param mintInterface            MintInterface from getMintInterface() - must have merkleContext
 * @param currentFreezeAuthority   Current freeze authority public key (must sign)
 * @param newFreezeAuthority       New freeze authority (or null to revoke)
 * @param payer                    Fee payer public key
 * @param validityProof            Validity proof for the compressed mint
 */
export function createUpdateFreezeAuthorityInstruction(
    mintInterface: MintInterface,
    currentFreezeAuthority: PublicKey,
    newFreezeAuthority: PublicKey | null,
    payer: PublicKey,
    validityProof: ValidityProofWithContext,
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

    const merkleContext = mintInterface.merkleContext;
    const outputQueue = getOutputQueue(merkleContext);

    const addressTreeInfo = getDefaultAddressTreeInfo();
    const data = encodeUpdateMintInstructionData({
        splMint: mintInterface.mintContext.splMint,
        addressTree: addressTreeInfo.tree,
        leafIndex: merkleContext.leafIndex,
        proveByIndex: true,
        rootIndex: validityProof.rootIndices[0],
        proof: validityProof.compressedProof,
        mintInterface,
        newAuthority: newFreezeAuthority,
        actionType: 'freezeAuthority',
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

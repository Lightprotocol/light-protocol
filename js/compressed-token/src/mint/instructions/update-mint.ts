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
import { MintInstructionData } from '../serde';
import {
    encodeMintActionInstructionData,
    MintActionCompressedInstructionData,
    Action,
    ExtensionInstructionData,
} from './mint-action-layout';

interface EncodeUpdateMintInstructionParams {
    addressTree: PublicKey;
    leafIndex: number;
    proveByIndex: boolean;
    rootIndex: number;
    proof: { a: number[]; b: number[]; c: number[] } | null;
    mintData: MintInstructionData;
    newAuthority: PublicKey | null;
    actionType: 'mintAuthority' | 'freezeAuthority';
}

function encodeUpdateMintInstructionData(
    params: EncodeUpdateMintInstructionParams,
): Buffer {
    const compressedAddress = deriveAddressV2(
        params.mintData.splMint.toBytes(),
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
    if (params.mintData.metadata) {
        extensions = [
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
        ];
    }

    const instructionData: MintActionCompressedInstructionData = {
        leafIndex: params.leafIndex,
        proveByIndex: params.proveByIndex,
        rootIndex: params.rootIndex,
        compressedAddress: Array.from(compressedAddress.toBytes()),
        tokenPoolBump: 0,
        tokenPoolIndex: 0,
        createMint: null,
        actions: [action],
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
            extensions,
        },
    };

    return encodeMintActionInstructionData(instructionData);
}

export function createUpdateMintAuthorityInstruction(
    mintSigner: PublicKey,
    currentMintAuthority: PublicKey,
    newMintAuthority: PublicKey | null,
    payer: PublicKey,
    validityProof: ValidityProofWithContext,
    merkleContext: MerkleContext,
    mintData: MintInstructionData,
    outputQueue: PublicKey,
): TransactionInstruction {
    const addressTreeInfo = getDefaultAddressTreeInfo();
    const data = encodeUpdateMintInstructionData({
        addressTree: addressTreeInfo.tree,
        leafIndex: merkleContext.leafIndex,
        proveByIndex: true,
        rootIndex: validityProof.rootIndices[0],
        proof: validityProof.compressedProof,
        mintData,
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

export function createUpdateFreezeAuthorityInstruction(
    mintSigner: PublicKey,
    currentFreezeAuthority: PublicKey,
    newFreezeAuthority: PublicKey | null,
    payer: PublicKey,
    validityProof: ValidityProofWithContext,
    merkleContext: MerkleContext,
    mintData: MintInstructionData,
    outputQueue: PublicKey,
): TransactionInstruction {
    const addressTreeInfo = getDefaultAddressTreeInfo();
    const data = encodeUpdateMintInstructionData({
        addressTree: addressTreeInfo.tree,
        leafIndex: merkleContext.leafIndex,
        proveByIndex: true,
        rootIndex: validityProof.rootIndices[0],
        proof: validityProof.compressedProof,
        mintData,
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

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
    TreeInfo,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../../program';
import { MintInstructionData } from '../serde';
import {
    encodeMintActionInstructionData,
    MintActionCompressedInstructionData,
} from './mint-action-layout';

interface EncodeMintToCTokenInstructionParams {
    addressTree: PublicKey;
    leafIndex: number;
    rootIndex: number;
    proof: { a: number[]; b: number[]; c: number[] } | null;
    mintData: MintInstructionData;
    recipientAccountIndex: number;
    amount: number | bigint;
}

function encodeMintToCTokenInstructionData(
    params: EncodeMintToCTokenInstructionParams,
): Buffer {
    const compressedAddress = deriveAddressV2(
        params.mintData.splMint.toBytes(),
        params.addressTree,
        CTOKEN_PROGRAM_ID,
    );

    // TokenMetadata extension not supported in mintTo instruction
    if (params.mintData.metadata) {
        throw new Error(
            'TokenMetadata extension not supported in mintTo instruction',
        );
    }

    const instructionData: MintActionCompressedInstructionData = {
        leafIndex: params.leafIndex,
        proveByIndex: true,
        rootIndex: params.rootIndex,
        compressedAddress: Array.from(compressedAddress.toBytes()),
        tokenPoolBump: 0,
        tokenPoolIndex: 0,
        createMint: null,
        actions: [
            {
                mintToCToken: {
                    accountIndex: params.recipientAccountIndex,
                    amount: BigInt(params.amount.toString()),
                },
            },
        ],
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
            extensions: null,
        },
    };

    return encodeMintActionInstructionData(instructionData);
}

export function createMintToInstruction(
    mintSigner: PublicKey,
    authority: PublicKey,
    payer: PublicKey,
    validityProof: ValidityProofWithContext,
    merkleContext: MerkleContext,
    mintData: MintInstructionData,
    outputStateTreeInfo: TreeInfo,
    tokensOutQueue: PublicKey,
    recipientAccount: PublicKey,
    amount: number | bigint,
): TransactionInstruction {
    const addressTreeInfo = getDefaultAddressTreeInfo();
    const data = encodeMintToCTokenInstructionData({
        addressTree: addressTreeInfo.tree,
        leafIndex: merkleContext.leafIndex,
        rootIndex: validityProof.rootIndices[0],
        proof: validityProof.compressedProof,
        mintData,
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
        {
            pubkey: outputStateTreeInfo.queue,
            isSigner: false,
            isWritable: true,
        },
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

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
} from './mint-action-layout';

interface EncodeCompressedMintToInstructionParams {
    addressTree: PublicKey;
    leafIndex: number;
    rootIndex: number;
    proof: { a: number[]; b: number[]; c: number[] } | null;
    mintData: MintInstructionData;
    recipients: Array<{ recipient: PublicKey; amount: number | bigint }>;
    tokenAccountVersion: number;
}

function encodeCompressedMintToInstructionData(
    params: EncodeCompressedMintToInstructionParams,
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
                mintToCompressed: {
                    tokenAccountVersion: params.tokenAccountVersion,
                    recipients: params.recipients.map(r => ({
                        recipient: r.recipient,
                        amount: BigInt(r.amount.toString()),
                    })),
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

// Keep old interface type for backwards compatibility export
export interface CreateMintToCompressedInstructionParams {
    mintSigner: PublicKey;
    authority: PublicKey;
    payer: PublicKey;
    validityProof: ValidityProofWithContext;
    merkleContext: MerkleContext;
    mintData: MintInstructionData;
    outputQueue: PublicKey;
    tokensOutQueue: PublicKey;
    recipients: Array<{ recipient: PublicKey; amount: number | bigint }>;
    tokenAccountVersion?: number;
}

/**
 * Create instruction for minting compressed tokens to compressed accounts.
 *
 * @param authority           Mint authority public key.
 * @param payer               Fee payer public key.
 * @param validityProof       Validity proof for the compressed mint.
 * @param merkleContext       Merkle context of the compressed mint.
 * @param mintData            Mint instruction data.
 * @param outputQueue         Output queue for state changes.
 * @param tokensOutQueue      Queue for token outputs.
 * @param recipients          Array of recipients with amounts.
 * @param tokenAccountVersion Token account version (default: 3).
 */
export function createMintToCompressedInstruction(
    authority: PublicKey,
    payer: PublicKey,
    validityProof: ValidityProofWithContext,
    merkleContext: MerkleContext,
    mintData: MintInstructionData,
    outputQueue: PublicKey,
    tokensOutQueue: PublicKey,
    recipients: Array<{ recipient: PublicKey; amount: number | bigint }>,
    tokenAccountVersion: number = 3,
): TransactionInstruction {
    const addressTreeInfo = getDefaultAddressTreeInfo();
    const data = encodeCompressedMintToInstructionData({
        addressTree: addressTreeInfo.tree,
        leafIndex: merkleContext.leafIndex,
        rootIndex: validityProof.rootIndices[0],
        proof: validityProof.compressedProof,
        mintData,
        recipients,
        tokenAccountVersion,
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

    return new TransactionInstruction({
        programId: CTOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

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
    getDefaultAddressTreeInfo,
    MerkleContext,
    TreeInfo,
    getOutputQueue,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../../program';
import { MintInstructionData } from '../layout/layout-mint';
import {
    encodeMintActionInstructionData,
    MintActionCompressedInstructionData,
} from '../layout/layout-mint-action';
import { TokenDataVersion } from '../../constants';

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
        maxTopUp: 65535,
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
                cmintDecompressed: params.mintData.cmintDecompressed,
                mint: params.mintData.splMint,
                mintSigner: Array.from(params.mintData.mintSigner),
                bump: params.mintData.bump,
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
    recipients: Array<{ recipient: PublicKey; amount: number | bigint }>;
    outputStateTreeInfo?: TreeInfo;
    tokenAccountVersion?: TokenDataVersion;
}

/**
 * Create instruction for minting tokens from a c-mint to compressed accounts.
 * To mint to onchain token accounts across SPL/T22/c-mints, use
 * {@link createMintToInterfaceInstruction} instead.
 *
 * @param authority             Mint authority public key.
 * @param payer                 Fee payer public key.
 * @param validityProof         Validity proof for the compressed mint.
 * @param merkleContext         Merkle context of the compressed mint.
 * @param mintData              Mint instruction data.
 * @param recipients            Array of recipients with amounts.
 * @param outputStateTreeInfo   Optional output state tree info. Uses merkle
 * context queue if not provided.
 * @param tokenAccountVersion   Token account version (default:
 * TokenDataVersion.ShaFlat).
 */
export function createMintToCompressedInstruction(
    authority: PublicKey,
    payer: PublicKey,
    validityProof: ValidityProofWithContext,
    merkleContext: MerkleContext,
    mintData: MintInstructionData,
    recipients: Array<{ recipient: PublicKey; amount: number | bigint }>,
    outputStateTreeInfo?: TreeInfo,
    tokenAccountVersion: TokenDataVersion = TokenDataVersion.ShaFlat,
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

    // Use outputStateTreeInfo.queue if provided, otherwise derive from merkleContext
    const outputQueue =
        outputStateTreeInfo?.queue ?? getOutputQueue(merkleContext);

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
        // Use same queue for tokens out
        { pubkey: outputQueue, isSigner: false, isWritable: true },
    ];

    return new TransactionInstruction({
        programId: CTOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

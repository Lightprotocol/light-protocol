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
import { MintInstructionData } from '../layout/layout-mint';
import {
    encodeMintActionInstructionData,
    MintActionCompressedInstructionData,
} from '../layout/layout-mint-action';

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
        maxTopUp: 0,
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
                cmintDecompressed: params.mintData.cmintDecompressed,
                mint: params.mintData.splMint,
                compressedAddress: Array.from(compressedAddress.toBytes()),
            },
            mintAuthority: params.mintData.mintAuthority,
            freezeAuthority: params.mintData.freezeAuthority,
            extensions: null,
        },
    };

    return encodeMintActionInstructionData(instructionData);
}

// Keep old interface type for backwards compatibility export
export interface CreateMintToInstructionParams {
    mintSigner: PublicKey;
    authority: PublicKey;
    payer: PublicKey;
    validityProof: ValidityProofWithContext;
    merkleContext: MerkleContext;
    mintData: MintInstructionData;
    outputStateTreeInfo: TreeInfo;
    tokensOutQueue: PublicKey;
    recipientAccount: PublicKey;
    amount: number | bigint;
}

/**
 * Create instruction for minting compressed tokens to an onchain token account.
 *
 * @param authority           Mint authority public key.
 * @param payer               Fee payer public key.
 * @param validityProof       Validity proof for the compressed mint.
 * @param merkleContext       Merkle context of the compressed mint.
 * @param mintData            Mint instruction data.
 * @param outputStateTreeInfo Output state tree info.
 * @param recipientAccount    Recipient onchain token account address.
 * @param amount              Amount to mint.
 */
export function createMintToInstruction(
    authority: PublicKey,
    payer: PublicKey,
    validityProof: ValidityProofWithContext,
    merkleContext: MerkleContext,
    mintData: MintInstructionData,
    outputStateTreeInfo: TreeInfo,
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
        // Note: tokensOutQueue is NOT included for MintToCToken-only actions.
        // MintToCToken mints to existing decompressed accounts, doesn't create
        // new compressed outputs so Rust expects no tokens_out_queue account.
    ];

    keys.push({ pubkey: recipientAccount, isSigner: false, isWritable: true });

    return new TransactionInstruction({
        programId: CTOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

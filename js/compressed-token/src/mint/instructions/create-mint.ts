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
    TreeInfo,
    AddressTreeInfo,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../../program';
import { findMintAddress } from '../../compressible/derivation';
import {
    encodeMintActionInstructionData,
    MintActionCompressedInstructionData,
    TokenMetadataInstructionData as TokenMetadataBorshData,
} from './mint-action-layout';

/**
 * Token metadata for creating a compressed mint
 * Uses strings for user-friendly input
 */
export interface TokenMetadataInstructionData {
    name: string;
    symbol: string;
    uri: string;
    updateAuthority?: PublicKey | null;
    additionalMetadata?: {
        key: string;
        value: string;
    }[];
}

/** @deprecated Use TokenMetadataInstructionData instead */
export type TokenMetadataInstructionDataInput = TokenMetadataInstructionData;

interface EncodeCreateMintInstructionParams {
    mintSigner: PublicKey;
    mintAuthority: PublicKey;
    freezeAuthority: PublicKey | null;
    decimals: number;
    addressTree: PublicKey;
    outputQueue: PublicKey;
    rootIndex: number;
    proof: { a: number[]; b: number[]; c: number[] } | null;
    metadata?: TokenMetadataInstructionData;
}

export function createTokenMetadata(
    name: string,
    symbol: string,
    uri: string,
    updateAuthority?: PublicKey | null,
): TokenMetadataInstructionData {
    return {
        name,
        symbol,
        uri,
        updateAuthority: updateAuthority ?? null,
    };
}

function encodeCreateMintInstructionData(
    params: EncodeCreateMintInstructionParams,
): Buffer {
    const [splMintPda] = findMintAddress(params.mintSigner);
    const compressedAddress = deriveAddressV2(
        splMintPda.toBytes(),
        params.addressTree,
        CTOKEN_PROGRAM_ID,
    );

    // Build extensions if metadata present
    let extensions: { tokenMetadata: TokenMetadataBorshData }[] | null = null;
    if (params.metadata) {
        extensions = [
            {
                tokenMetadata: {
                    updateAuthority: params.metadata.updateAuthority ?? null,
                    name: Buffer.from(params.metadata.name),
                    symbol: Buffer.from(params.metadata.symbol),
                    uri: Buffer.from(params.metadata.uri),
                    additionalMetadata: null,
                },
            },
        ];
    }

    const instructionData: MintActionCompressedInstructionData = {
        leafIndex: 0,
        proveByIndex: false,
        rootIndex: params.rootIndex,
        compressedAddress: Array.from(compressedAddress.toBytes()),
        tokenPoolBump: 0,
        tokenPoolIndex: 0,
        createMint: {
            readOnlyAddressTrees: [0, 0, 0, 0],
            readOnlyAddressTreeRootIndices: [0, 0, 0, 0],
        },
        actions: [], // No actions for create mint
        proof: params.proof,
        cpiContext: null,
        mint: {
            supply: BigInt(0),
            decimals: params.decimals,
            metadata: {
                version: 3,
                splMintInitialized: false,
                mint: splMintPda,
            },
            mintAuthority: params.mintAuthority,
            freezeAuthority: params.freezeAuthority,
            extensions,
        },
    };

    return encodeMintActionInstructionData(instructionData);
}

// Keep old interface type for backwards compatibility export
export interface CreateMintInstructionParams {
    mintSigner: PublicKey;
    decimals: number;
    mintAuthority: PublicKey;
    freezeAuthority: PublicKey | null;
    payer: PublicKey;
    validityProof: ValidityProofWithContext;
    metadata?: TokenMetadataInstructionData;
    addressTreeInfo: AddressTreeInfo;
    outputStateTreeInfo: TreeInfo;
}

/**
 * Create instruction for initializing a compressed token mint.
 *
 * @param mintSigner          Mint signer keypair public key.
 * @param decimals            Number of decimals for the mint.
 * @param mintAuthority       Mint authority public key.
 * @param freezeAuthority     Optional freeze authority public key.
 * @param payer               Fee payer public key.
 * @param validityProof       Validity proof for the compressed account.
 * @param addressTreeInfo     Address tree info for the mint.
 * @param outputStateTreeInfo Output state tree info.
 * @param metadata            Optional token metadata.
 */
export function createMintInstruction(
    mintSigner: PublicKey,
    decimals: number,
    mintAuthority: PublicKey,
    freezeAuthority: PublicKey | null,
    payer: PublicKey,
    validityProof: ValidityProofWithContext,
    addressTreeInfo: AddressTreeInfo,
    outputStateTreeInfo: TreeInfo,
    metadata?: TokenMetadataInstructionData,
): TransactionInstruction {
    const data = encodeCreateMintInstructionData({
        mintSigner,
        mintAuthority,
        freezeAuthority,
        decimals,
        addressTree: addressTreeInfo.tree,
        outputQueue: outputStateTreeInfo.queue,
        rootIndex: validityProof.rootIndices[0],
        proof: validityProof.compressedProof,
        metadata,
    });

    const sys = defaultStaticAccountsStruct();
    const keys = [
        {
            pubkey: LightSystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: mintSigner, isSigner: true, isWritable: false },
        { pubkey: mintAuthority, isSigner: true, isWritable: false },
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
            pubkey: addressTreeInfo.tree,
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

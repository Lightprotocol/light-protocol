import {
    PublicKey,
    SystemProgram,
    TransactionInstruction,
} from '@solana/web3.js';
import { Buffer } from 'buffer';
import {
    ValidityProofWithContext,
    LIGHT_TOKEN_PROGRAM_ID,
    LightSystemProgram,
    defaultStaticAccountsStruct,
    TreeInfo,
    AddressTreeInfo,
    ValidityProof,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../../program';
import { findMintAddress } from '../derivation';
import {
    AdditionalMetadata,
    encodeMintActionInstructionData,
    MintActionCompressedInstructionData,
    TokenMetadataLayoutData as TokenMetadataBorshData,
} from '../layout/layout-mint-action';
import {
    MAX_TOP_UP,
    TokenDataVersion,
    LIGHT_TOKEN_CONFIG,
    LIGHT_TOKEN_RENT_SPONSOR,
} from '../../constants';

/**
 * Token metadata for creating a c-token mint.
 */
export interface TokenMetadataInstructionData {
    name: string;
    symbol: string;
    uri: string;
    updateAuthority?: PublicKey | null;
    additionalMetadata: AdditionalMetadata[] | null;
}

export interface EncodeCreateMintInstructionParams {
    mintSigner: PublicKey;
    mintAuthority: PublicKey;
    freezeAuthority: PublicKey | null;
    decimals: number;
    addressTree: PublicKey;
    outputQueue: PublicKey;
    rootIndex: number;
    proof: ValidityProof | null;
    metadata?: TokenMetadataInstructionData;
    maxTopUp?: number;
}

export function createTokenMetadata(
    name: string,
    symbol: string,
    uri: string,
    updateAuthority?: PublicKey | null,
    additionalMetadata: AdditionalMetadata[] | null = null,
): TokenMetadataInstructionData {
    return {
        name,
        symbol,
        uri,
        updateAuthority: updateAuthority ?? null,
        additionalMetadata: additionalMetadata ?? null,
    };
}

/**
 * Validate and normalize proof arrays to ensure correct sizes for Borsh serialization.
 * The compressed proof must have exactly: a[32], b[64], c[32] bytes.
 * @internal
 */
function validateProofArrays(
    proof: ValidityProof | null,
): ValidityProof | null {
    if (!proof) return null;

    // Validate array sizes
    if (proof.a.length !== 32) {
        throw new Error(
            `Invalid proof.a length: expected 32, got ${proof.a.length}`,
        );
    }
    if (proof.b.length !== 64) {
        throw new Error(
            `Invalid proof.b length: expected 64, got ${proof.b.length}`,
        );
    }
    if (proof.c.length !== 32) {
        throw new Error(
            `Invalid proof.c length: expected 32, got ${proof.c.length}`,
        );
    }

    return proof;
}

/** @internal */
export function encodeCreateMintInstructionData(
    params: EncodeCreateMintInstructionParams,
): Buffer {
    const [splMintPda, bump] = findMintAddress(params.mintSigner);

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
                    additionalMetadata: params.metadata.additionalMetadata,
                },
            },
        ];
    }

    // Validate proof arrays before encoding
    const validatedProof = validateProofArrays(params.proof);

    /** TODO: check leafIndex */
    const instructionData: MintActionCompressedInstructionData = {
        leafIndex: 0,
        proveByIndex: false,
        rootIndex: params.rootIndex,
        maxTopUp: params.maxTopUp ?? MAX_TOP_UP,
        createMint: {
            readOnlyAddressTrees: [0, 0, 0, 0],
            readOnlyAddressTreeRootIndices: [0, 0, 0, 0],
        },
        actions: [
            {
                decompressMint: {
                    rentPayment: 16,
                    writeTopUp: 766,
                },
            },
        ],
        proof: validatedProof,
        cpiContext: null,
        mint: {
            supply: BigInt(0),
            decimals: params.decimals,
            metadata: {
                version: TokenDataVersion.ShaFlat,
                cmintDecompressed: false,
                mint: splMintPda,
                mintSigner: Array.from(params.mintSigner.toBytes()),
                bump,
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
 * Create instruction for initializing a c-token mint.
 *
 * @param mintSigner          Mint signer keypair public key.
 * @param decimals            Number of decimals for the mint.
 * @param mintAuthority       Mint authority public key.
 * @param freezeAuthority     Optional freeze authority public key.
 * @param payer               Fee payer public key.
 * @param validityProof       Validity proof for the mint account.
 * @param addressTreeInfo     Address tree info for the mint.
 * @param outputStateTreeInfo Output state tree info.
 * @param metadata            Optional token metadata.
 * @param maxTopUp             Optional cap on rent top-up (units of 1k lamports; default no cap)
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
    maxTopUp?: number,
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
        maxTopUp,
    });

    return buildCreateMintIx(
        mintSigner,
        mintAuthority,
        payer,
        outputStateTreeInfo,
        addressTreeInfo,
        data,
    );
}

/** @internal */
function buildCreateMintIx(
    mintSigner: PublicKey,
    mintAuthority: PublicKey,
    payer: PublicKey,
    outputStateTreeInfo: TreeInfo,
    addressTreeInfo: AddressTreeInfo,
    data: Buffer,
): TransactionInstruction {
    const sys = defaultStaticAccountsStruct();
    const [splMintPda] = findMintAddress(mintSigner);
    const keys = [
        {
            pubkey: LightSystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: mintSigner, isSigner: true, isWritable: false },
        { pubkey: mintAuthority, isSigner: true, isWritable: false },
        {
            pubkey: LIGHT_TOKEN_CONFIG,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: splMintPda, isSigner: false, isWritable: true },
        {
            pubkey: LIGHT_TOKEN_RENT_SPONSOR,
            isSigner: false,
            isWritable: true,
        },
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
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

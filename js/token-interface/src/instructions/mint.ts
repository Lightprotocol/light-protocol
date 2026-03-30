import {
    AddressTreeInfo,
    DerivationMode,
    LIGHT_TOKEN_PROGRAM_ID,
    LightSystemProgram,
    TreeInfo,
    ValidityProof,
    ValidityProofWithContext,
    defaultStaticAccountsStruct,
    deriveAddressV2,
    getBatchAddressTreeInfo,
    getDefaultAddressTreeInfo,
    selectStateTreeInfo,
} from '@lightprotocol/stateless.js';
import {
    PublicKey,
    SystemProgram,
    TransactionInstruction,
} from '@solana/web3.js';
import {
    MINT_SIZE,
    TOKEN_2022_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    createInitializeMint2Instruction,
    createMintToInstruction as createSplMintToInstruction,
} from '@solana/spl-token';
import { Buffer } from 'buffer';
import type {
    CreateMintInstructionsInput,
    CreateMintToInstructionsInput,
    CreateRawLightMintInstructionInput,
    CreateRawMintInstructionInput,
    CreateRawMintToInstructionInput,
    TokenMetadataInput,
} from '../types';
import { toBigIntAmount } from '../helpers';
import {
    LIGHT_TOKEN_CONFIG,
    LIGHT_TOKEN_RENT_SPONSOR,
    MAX_TOP_UP,
    TokenDataVersion,
    deriveCpiAuthorityPda,
} from '../constants';
import { createSplInterfaceInstruction } from './spl-interface';
import { toInstructionPlan } from './_plan';
import {
    AdditionalMetadata,
    MintActionCompressedInstructionData,
    TokenMetadataLayoutData,
    encodeMintActionInstructionData,
} from './layout/layout-mint-action';

const LIGHT_TOKEN_MINT_TO_DISCRIMINATOR = 7;
const COMPRESSED_MINT_SEED = Buffer.from('compressed_mint');

function assertSupportedMintProgram(programId: PublicKey): void {
    if (
        !programId.equals(TOKEN_PROGRAM_ID) &&
        !programId.equals(TOKEN_2022_PROGRAM_ID)
    ) {
        throw new Error(
            `Unsupported token program ${programId.toBase58()} for createMintInstructions. ` +
                'Use TOKEN_PROGRAM_ID or TOKEN_2022_PROGRAM_ID.',
        );
    }
}

/**
 * Create initialize-mint instruction for SPL/T22 mints.
 */
export function createMintInstruction({
    mint,
    decimals,
    mintAuthority,
    freezeAuthority = null,
    tokenProgramId = TOKEN_PROGRAM_ID,
}: CreateRawMintInstructionInput): TransactionInstruction {
    assertSupportedMintProgram(tokenProgramId);
    return createInitializeMint2Instruction(
        mint,
        decimals,
        mintAuthority,
        freezeAuthority,
        tokenProgramId,
    );
}

function findMintAddress(mintSigner: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
        [COMPRESSED_MINT_SEED, mintSigner.toBuffer()],
        LIGHT_TOKEN_PROGRAM_ID,
    );
}

function deriveLightMintAddress(
    mintSigner: PublicKey,
    addressTreeInfo: AddressTreeInfo,
): number[] {
    const [mintPda] = findMintAddress(mintSigner);
    return Array.from(
        deriveAddressV2(
            mintPda.toBytes(),
            addressTreeInfo.tree,
            LIGHT_TOKEN_PROGRAM_ID,
        ).toBytes(),
    );
}

function validateProofArrays(proof: ValidityProof | null): ValidityProof | null {
    if (!proof) return null;
    if (proof.a.length !== 32 || proof.b.length !== 64 || proof.c.length !== 32) {
        throw new Error('Invalid compressed proof shape for light mint creation.');
    }
    return proof;
}

function toMetadataLayoutData(
    metadata?: TokenMetadataInput,
): TokenMetadataLayoutData | null {
    if (!metadata) return null;
    const additionalMetadata: AdditionalMetadata[] | null =
        metadata.additionalMetadata?.map(entry => ({
            key: Buffer.from(entry.key),
            value: Buffer.from(entry.value),
        })) ?? null;

    return {
        updateAuthority: metadata.updateAuthority ?? null,
        name: Buffer.from(metadata.name),
        symbol: Buffer.from(metadata.symbol),
        uri: Buffer.from(metadata.uri),
        additionalMetadata,
    };
}

/**
 * Convenience helper to build token metadata input.
 */
export function createTokenMetadata({
    name,
    symbol,
    uri,
    updateAuthority = null,
    additionalMetadata = null,
}: TokenMetadataInput): TokenMetadataInput {
    return {
        name,
        symbol,
        uri,
        updateAuthority,
        additionalMetadata,
    };
}

/**
 * Build raw light-mint creation instruction (MintAction + decompressMint action).
 */
export function createLightMintInstruction({
    mintSigner,
    decimals,
    mintAuthority,
    freezeAuthority = null,
    payer,
    validityProof,
    addressTreeInfo,
    outputStateTreeInfo,
    tokenMetadata,
    maxTopUp = MAX_TOP_UP,
}: CreateRawLightMintInstructionInput): TransactionInstruction {
    if (validityProof.rootIndices.length === 0) {
        throw new Error('Missing root index for light mint creation proof.');
    }

    const [mintPda, bump] = findMintAddress(mintSigner);
    const metadataLayout = toMetadataLayoutData(tokenMetadata);
    const sys = defaultStaticAccountsStruct();

    const instructionData: MintActionCompressedInstructionData = {
        leafIndex: 0,
        proveByIndex: false,
        rootIndex: validityProof.rootIndices[0],
        maxTopUp,
        createMint: {
            readOnlyAddressTrees: [0, 0, 0, 0],
            readOnlyAddressTreeRootIndices: [0, 0, 0, 0],
        },
        actions: [{ decompressMint: { rentPayment: 16, writeTopUp: 766 } }],
        proof: validateProofArrays(validityProof.compressedProof),
        cpiContext: null,
        mint: {
            supply: BigInt(0),
            decimals,
            metadata: {
                version: TokenDataVersion.ShaFlat,
                cmintDecompressed: false,
                mint: mintPda,
                mintSigner: Array.from(mintSigner.toBytes()),
                bump,
            },
            mintAuthority,
            freezeAuthority,
            extensions: metadataLayout ? [{ tokenMetadata: metadataLayout }] : null,
        },
    };

    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys: [
            {
                pubkey: LightSystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
            { pubkey: mintSigner, isSigner: true, isWritable: false },
            { pubkey: mintAuthority, isSigner: true, isWritable: false },
            { pubkey: LIGHT_TOKEN_CONFIG, isSigner: false, isWritable: false },
            { pubkey: mintPda, isSigner: false, isWritable: true },
            {
                pubkey: LIGHT_TOKEN_RENT_SPONSOR,
                isSigner: false,
                isWritable: true,
            },
            { pubkey: payer, isSigner: true, isWritable: true },
            {
                pubkey: deriveCpiAuthorityPda(),
                isSigner: false,
                isWritable: false,
            },
            { pubkey: sys.registeredProgramPda, isSigner: false, isWritable: false },
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
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: outputStateTreeInfo.queue,
                isSigner: false,
                isWritable: true,
            },
            { pubkey: addressTreeInfo.tree, isSigner: false, isWritable: true },
        ],
        data: encodeMintActionInstructionData(instructionData),
    });
}

/**
 * Build canonical mint creation flow.
 *
 * Defaults to SPL (`TOKEN_PROGRAM_ID`) for broad compatibility.
 * To create other mint types, pass `tokenProgramId` explicitly:
 * - SPL: `TOKEN_PROGRAM_ID` (create account + init mint + SPL interface)
 * - Token-2022: `TOKEN_2022_PROGRAM_ID` (same flow with T22 program)
 * - light-token: `LIGHT_TOKEN_PROGRAM_ID` (MintAction light mint flow)
 */
export async function createMintInstructions({
    rpc,
    payer,
    keypair,
    decimals,
    mintAuthority,
    freezeAuthority = null,
    tokenProgramId = TOKEN_PROGRAM_ID,
    mintSize = MINT_SIZE,
    rentExemptBalance,
    splInterfaceIndex = 0,
    tokenMetadata,
    outputStateTreeInfo,
    addressTreeInfo,
    maxTopUp,
}: CreateMintInstructionsInput): Promise<TransactionInstruction[]> {
    const keypairPubkey = keypair.publicKey;

    if (tokenProgramId.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        const resolvedAddressTreeInfo = addressTreeInfo ?? getBatchAddressTreeInfo();
        const defaultAddressTreeInfo = getDefaultAddressTreeInfo();
        if (!resolvedAddressTreeInfo.tree.equals(defaultAddressTreeInfo.tree)) {
            throw new Error(
                `addressTreeInfo ${resolvedAddressTreeInfo.tree.toBase58()} must match default ${defaultAddressTreeInfo.tree.toBase58()}.`,
            );
        }

        const resolvedOutputStateTreeInfo =
            outputStateTreeInfo ?? selectStateTreeInfo(await rpc.getStateTreeInfos());

        const compressedMintAddress = deriveLightMintAddress(
            keypairPubkey,
            resolvedAddressTreeInfo,
        );
        const validityProof = await rpc.getValidityProofV2(
            [],
            [
                {
                    address: Uint8Array.from(compressedMintAddress),
                    treeInfo: resolvedAddressTreeInfo,
                },
            ],
            DerivationMode.standard,
        );

        return [
            createLightMintInstruction({
                mintSigner: keypairPubkey,
                decimals,
                mintAuthority,
                freezeAuthority,
                payer,
                validityProof,
                addressTreeInfo: resolvedAddressTreeInfo,
                outputStateTreeInfo: resolvedOutputStateTreeInfo,
                tokenMetadata,
                maxTopUp,
            }),
        ];
    }

    assertSupportedMintProgram(tokenProgramId);

    const lamports =
        rentExemptBalance ??
        (await rpc.getMinimumBalanceForRentExemption(mintSize));

    const createMintAccountInstruction = SystemProgram.createAccount({
        fromPubkey: payer,
        lamports,
        newAccountPubkey: keypairPubkey,
        programId: tokenProgramId,
        space: mintSize,
    });

    return [
        createMintAccountInstruction,
        createMintInstruction({
            mint: keypairPubkey,
            decimals,
            mintAuthority,
            freezeAuthority,
            tokenProgramId,
        }),
        createSplInterfaceInstruction({
            feePayer: payer,
            mint: keypairPubkey,
            index: splInterfaceIndex,
            tokenProgramId,
        }),
    ];
}

/**
 * Create mint-to instruction using SPL/T22/light-token semantics.
 *
 * Defaults to SPL (`TOKEN_PROGRAM_ID`) for broad compatibility.
 * To use other mint types, pass `tokenProgramId` explicitly:
 * - SPL: `TOKEN_PROGRAM_ID`
 * - Token-2022: `TOKEN_2022_PROGRAM_ID`
 * - light-token: `LIGHT_TOKEN_PROGRAM_ID`
 */
export function createMintToInstruction({
    mint,
    destination,
    authority,
    amount,
    payer,
    tokenProgramId = TOKEN_PROGRAM_ID,
    multiSigners = [],
    maxTopUp,
}: CreateRawMintToInstructionInput): TransactionInstruction {
    const amountBigInt = toBigIntAmount(amount);

    if (
        tokenProgramId.equals(TOKEN_PROGRAM_ID) ||
        tokenProgramId.equals(TOKEN_2022_PROGRAM_ID)
    ) {
        if (maxTopUp !== undefined) {
            throw new Error(
                'maxTopUp is only supported for LIGHT_TOKEN_PROGRAM_ID mint-to.',
            );
        }
        return createSplMintToInstruction(
            mint,
            destination,
            authority,
            amountBigInt,
            multiSigners,
            tokenProgramId,
        );
    }

    if (!tokenProgramId.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        throw new Error(
            `Unsupported token program ${tokenProgramId.toBase58()} for mint-to.`,
        );
    }
    if (multiSigners.length > 0) {
        throw new Error(
            'multiSigners are only supported for SPL/Token-2022 mint-to.',
        );
    }

    const feePayer =
        payer && !payer.equals(authority)
            ? { pubkey: payer, isSigner: true, isWritable: true }
            : null;
    const authorityWritable = maxTopUp !== undefined && feePayer === null;
    const data = Buffer.alloc(maxTopUp !== undefined ? 11 : 9);
    data.writeUInt8(LIGHT_TOKEN_MINT_TO_DISCRIMINATOR, 0);
    data.writeBigUInt64LE(amountBigInt, 1);
    if (maxTopUp !== undefined) {
        data.writeUInt16LE(maxTopUp, 9);
    }

    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys: [
            { pubkey: mint, isSigner: false, isWritable: true },
            { pubkey: destination, isSigner: false, isWritable: true },
            { pubkey: authority, isSigner: true, isWritable: authorityWritable },
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
            ...(feePayer ? [feePayer] : []),
        ],
        data,
    });
}

/**
 * Build one mint-to instruction with SPL default.
 *
 * This interface supports all 3 mint types. Pass `tokenProgramId` explicitly
 * when targeting Token-2022 or light-token mints.
 */
export async function createMintToInstructions({
    mint,
    destination,
    authority,
    amount,
    payer,
    tokenProgramId,
    multiSigners,
    maxTopUp,
}: CreateMintToInstructionsInput): Promise<TransactionInstruction[]> {
    return [
        createMintToInstruction({
            mint,
            destination,
            authority,
            amount,
            payer,
            tokenProgramId: tokenProgramId ?? TOKEN_PROGRAM_ID,
            multiSigners,
            maxTopUp,
        }),
    ];
}

export async function createMintInstructionPlan(input: CreateMintInstructionsInput) {
    return toInstructionPlan(await createMintInstructions(input));
}

export async function createMintToInstructionPlan(
    input: CreateMintToInstructionsInput,
) {
    return toInstructionPlan(await createMintToInstructions(input));
}

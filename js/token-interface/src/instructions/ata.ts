import {
    PublicKey,
    SystemProgram,
    TransactionInstruction,
} from '@solana/web3.js';
import { Buffer } from 'buffer';
import {
    TOKEN_PROGRAM_ID,
    createAssociatedTokenAccountInstruction as createSplAssociatedTokenAccountInstruction,
    createAssociatedTokenAccountIdempotentInstruction as createSplAssociatedTokenAccountIdempotentInstruction,
} from '@solana/spl-token';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { struct, u8, u32, option, vec, array } from '@coral-xyz/borsh';
import { LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR } from '../constants';
import { getAtaProgramId } from '../read/ata-utils';
import { getAtaAddress } from '../read';
import type { CreateRawAtaInstructionInput } from '../types';
import { toInstructionPlan } from './_plan';

const CREATE_ASSOCIATED_TOKEN_ACCOUNT_DISCRIMINATOR = Buffer.from([100]);
const CREATE_ASSOCIATED_TOKEN_ACCOUNT_IDEMPOTENT_DISCRIMINATOR = Buffer.from([
    102,
]);

// Matches Rust CompressToPubkey struct
const CompressToPubkeyLayout = struct([
    u8('bump'),
    array(u8(), 32, 'programId'),
    vec(vec(u8()), 'seeds'),
]);

// Matches Rust CompressibleExtensionInstructionData struct
// From: program-libs/token-interface/src/instructions/extensions/compressible.rs
const CompressibleExtensionInstructionDataLayout = struct([
    u8('tokenAccountVersion'),
    u8('rentPayment'),
    u8('compressionOnly'),
    u32('writeTopUp'),
    option(CompressToPubkeyLayout, 'compressToAccountPubkey'),
]);

const CreateAssociatedTokenAccountInstructionDataLayout = struct([
    option(CompressibleExtensionInstructionDataLayout, 'compressibleConfig'),
]);

export interface CompressToPubkey {
    bump: number;
    programId: number[];
    seeds: number[][];
}

export interface CompressibleConfig {
    tokenAccountVersion: number;
    rentPayment: number;
    compressionOnly: number;
    writeTopUp: number;
    compressToAccountPubkey?: CompressToPubkey | null;
}

export interface CreateAssociatedLightTokenAccountParams {
    compressibleConfig?: CompressibleConfig | null;
}

/**
 * Default compressible config for light-token ATAs - matches Rust SDK defaults.
 *
 * - tokenAccountVersion: 3 (ShaFlat) - latest hashing scheme
 * - rentPayment: 16 - prepay 16 epochs (~24 hours rent)
 * - compressionOnly: 1 - required for ATAs
 * - writeTopUp: 766 - per-write top-up (~2 epochs rent) when rent < 2 epochs
 * - compressToAccountPubkey: null - required for ATAs
 *
 * Cost breakdown at associated token account creation:
 * - Rent sponsor PDA (LIGHT_TOKEN_RENT_SPONSOR) pays: rent exemption (~890,880 lamports)
 * - Fee payer pays: compression_cost (11K) + 16 epochs rent (~6,400) = ~17,400 lamports + tx fees
 *
 * Per-write top-up (transfers):
 * - When account rent is below 2 epochs, fee payer pays 766 lamports top-up
 * - This keeps the account perpetually funded when actively used
 *
 * Rent calculation (272-byte compressible lightToken account):
 * - rent_per_epoch = base_rent (128) + bytes * rent_per_byte (272 * 1) = 400 lamports
 * - 16 epochs = 16 * 400 = 6,400 lamports (24 hours)
 * - 2 epochs = 2 * 400 = 800 lamports (~3 hours, writeTopUp = 766 is conservative)
 *
 * Account size breakdown (272 bytes):
 * - 165 bytes: SPL token base layout
 * - 1 byte: account_type discriminator
 * - 1 byte: Option discriminator for extensions
 * - 4 bytes: Vec length prefix
 * - 1 byte: extension type discriminant
 * - 4 bytes: CompressibleExtension header (decimals_option, decimals, compression_only, is_ata)
 * - 96 bytes: CompressionInfo struct
 */
export const DEFAULT_COMPRESSIBLE_CONFIG: CompressibleConfig = {
    tokenAccountVersion: 3, // ShaFlat (latest hashing scheme)
    rentPayment: 16, // 16 epochs (~24 hours) - matches Rust SDK
    compressionOnly: 1, // Required for ATAs
    writeTopUp: 766, // Per-write top-up (~2 epochs) - matches Rust SDK
    compressToAccountPubkey: null, // Required null for ATAs
};

/** @internal */
function getAssociatedLightTokenAddress(
    owner: PublicKey,
    mint: PublicKey,
): PublicKey {
    return PublicKey.findProgramAddressSync(
        [owner.toBuffer(), LIGHT_TOKEN_PROGRAM_ID.toBuffer(), mint.toBuffer()],
        LIGHT_TOKEN_PROGRAM_ID,
    )[0];
}

/** @internal */
function encodeCreateAssociatedLightTokenAccountData(
    params: CreateAssociatedLightTokenAccountParams,
    idempotent: boolean,
): Buffer {
    const discriminator = idempotent
        ? CREATE_ASSOCIATED_TOKEN_ACCOUNT_IDEMPOTENT_DISCRIMINATOR
        : CREATE_ASSOCIATED_TOKEN_ACCOUNT_DISCRIMINATOR;
    const payload = { compressibleConfig: params.compressibleConfig || null };
    let size = 64;

    for (;;) {
        const buffer = Buffer.alloc(size);
        try {
            const len = CreateAssociatedTokenAccountInstructionDataLayout.encode(
                payload,
                buffer,
            );
            return Buffer.concat([discriminator, buffer.subarray(0, len)]);
        } catch (error) {
            if (!(error instanceof RangeError) || size >= 4096) {
                throw error;
            }
            size *= 2;
        }
    }
}

export interface CreateAssociatedLightTokenAccountInstructionParams {
    feePayer?: PublicKey;
    owner: PublicKey;
    mint: PublicKey;
    compressibleConfig?: CompressibleConfig | null;
    configAccount?: PublicKey;
    rentPayerPda?: PublicKey;
}

/**
 * Create instruction for creating an associated light-token account.
 * Uses the default rent sponsor PDA by default.
 *
 * @param input                    Associated light-token account input.
 * @param input.feePayer           Optional fee payer public key. Defaults to owner.
 * @param input.owner              Owner of the associated token account.
 * @param input.mint               Mint address.
 * @param input.compressibleConfig Compressible configuration (defaults to rent sponsor config).
 * @param input.configAccount      Config account (defaults to LIGHT_TOKEN_CONFIG).
 * @param input.rentPayerPda       Rent payer PDA (defaults to LIGHT_TOKEN_RENT_SPONSOR).
 */
export function createAssociatedLightTokenAccountInstruction(
    {
        feePayer,
        owner,
        mint,
        compressibleConfig = DEFAULT_COMPRESSIBLE_CONFIG,
        configAccount = LIGHT_TOKEN_CONFIG,
        rentPayerPda = LIGHT_TOKEN_RENT_SPONSOR,
    }: CreateAssociatedLightTokenAccountInstructionParams,
): TransactionInstruction {
    const effectiveFeePayer = feePayer ?? owner;
    const associatedTokenAccount = getAssociatedLightTokenAddress(owner, mint);

    const data = encodeCreateAssociatedLightTokenAccountData(
        {
            compressibleConfig,
        },
        false,
    );

    // Account order per Rust processor:
    // 0. owner (non-mut, non-signer)
    // 1. mint (non-mut, non-signer)
    // 2. fee_payer (signer, mut)
    // 3. associated_token_account (mut)
    // 4. system_program
    // Optional (only when compressibleConfig is non-null):
    // 5. config account
    // 6. rent_payer PDA
    const keys: {
        pubkey: PublicKey;
        isSigner: boolean;
        isWritable: boolean;
    }[] = [
        { pubkey: owner, isSigner: false, isWritable: false },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: effectiveFeePayer, isSigner: true, isWritable: true },
        {
            pubkey: associatedTokenAccount,
            isSigner: false,
            isWritable: true,
        },
        {
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
    ];

    if (compressibleConfig) {
        keys.push(
            { pubkey: configAccount, isSigner: false, isWritable: false },
            { pubkey: rentPayerPda, isSigner: false, isWritable: true },
        );
    }

    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

/**
 * Create idempotent instruction for creating an associated light-token account.
 * Uses the default rent sponsor PDA by default.
 *
 * @param input                    Associated light-token account input.
 * @param input.feePayer           Optional fee payer public key. Defaults to owner.
 * @param input.owner              Owner of the associated token account.
 * @param input.mint               Mint address.
 * @param input.compressibleConfig Compressible configuration (defaults to rent sponsor config).
 * @param input.configAccount      Config account (defaults to LIGHT_TOKEN_CONFIG).
 * @param input.rentPayerPda       Rent payer PDA (defaults to LIGHT_TOKEN_RENT_SPONSOR).
 */
export function createAssociatedLightTokenAccountIdempotentInstruction(
    {
        feePayer,
        owner,
        mint,
        compressibleConfig = DEFAULT_COMPRESSIBLE_CONFIG,
        configAccount = LIGHT_TOKEN_CONFIG,
        rentPayerPda = LIGHT_TOKEN_RENT_SPONSOR,
    }: CreateAssociatedLightTokenAccountInstructionParams,
): TransactionInstruction {
    const effectiveFeePayer = feePayer ?? owner;
    const associatedTokenAccount = getAssociatedLightTokenAddress(owner, mint);

    const data = encodeCreateAssociatedLightTokenAccountData(
        {
            compressibleConfig,
        },
        true,
    );

    const keys: {
        pubkey: PublicKey;
        isSigner: boolean;
        isWritable: boolean;
    }[] = [
        { pubkey: owner, isSigner: false, isWritable: false },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: effectiveFeePayer, isSigner: true, isWritable: true },
        {
            pubkey: associatedTokenAccount,
            isSigner: false,
            isWritable: true,
        },
        {
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
    ];

    if (compressibleConfig) {
        keys.push(
            { pubkey: configAccount, isSigner: false, isWritable: false },
            { pubkey: rentPayerPda, isSigner: false, isWritable: true },
        );
    }

    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

/**
 * light-token-specific config for createAssociatedTokenAccountInstruction
 */
export interface LightTokenConfig {
    compressibleConfig?: CompressibleConfig | null;
    configAccount?: PublicKey;
    rentPayerPda?: PublicKey;
}

export interface CreateAssociatedTokenAccountInstructionInput {
    payer?: PublicKey;
    associatedToken: PublicKey;
    owner: PublicKey;
    mint: PublicKey;
    programId?: PublicKey;
    associatedTokenProgramId?: PublicKey;
    lightTokenConfig?: LightTokenConfig;
}

/**
 * Create instruction for creating an associated token account (SPL, Token-2022,
 * or light-token).
 *
 * @param input                          Associated token account input.
 * @param input.payer                    Fee payer public key.
 * @param input.associatedToken          Associated token account address.
 * @param input.owner                    Owner of the associated token account.
 * @param input.mint                     Mint address.
 * @param input.programId                Token program ID (default: TOKEN_PROGRAM_ID).
 * @param input.associatedTokenProgramId Associated token program ID.
 * @param input.lightTokenConfig         Optional light-token-specific configuration.
 */
function createAssociatedTokenAccountInstruction({
    payer,
    associatedToken,
    owner,
    mint,
    programId = TOKEN_PROGRAM_ID,
    associatedTokenProgramId,
    lightTokenConfig,
}: CreateAssociatedTokenAccountInstructionInput): TransactionInstruction {
    const effectivePayer = payer ?? owner;
    const effectiveAssociatedTokenProgramId =
        associatedTokenProgramId ?? getAtaProgramId(programId);

    if (programId.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        return createAssociatedLightTokenAccountInstruction({
            feePayer: effectivePayer,
            owner,
            mint,
            compressibleConfig: lightTokenConfig?.compressibleConfig,
            configAccount: lightTokenConfig?.configAccount,
            rentPayerPda: lightTokenConfig?.rentPayerPda,
        });
    } else {
        return createSplAssociatedTokenAccountInstruction(
            effectivePayer,
            associatedToken,
            owner,
            mint,
            programId,
            effectiveAssociatedTokenProgramId,
        );
    }
}

/**
 * Create idempotent instruction for creating an associated token account (SPL,
 * Token-2022, or light-token).
 *
 * @param input                          Associated token account input.
 * @param input.payer                    Fee payer public key.
 * @param input.associatedToken          Associated token account address.
 * @param input.owner                    Owner of the associated token account.
 * @param input.mint                     Mint address.
 * @param input.programId                Token program ID (default: TOKEN_PROGRAM_ID).
 * @param input.associatedTokenProgramId Associated token program ID.
 * @param input.lightTokenConfig         Optional light-token-specific configuration.
 */
function createAssociatedTokenAccountIdempotentInstruction({
    payer,
    associatedToken,
    owner,
    mint,
    programId = TOKEN_PROGRAM_ID,
    associatedTokenProgramId,
    lightTokenConfig,
}: CreateAssociatedTokenAccountInstructionInput): TransactionInstruction {
    const effectivePayer = payer ?? owner;
    const effectiveAssociatedTokenProgramId =
        associatedTokenProgramId ?? getAtaProgramId(programId);

    if (programId.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        return createAssociatedLightTokenAccountIdempotentInstruction({
            feePayer: effectivePayer,
            owner,
            mint,
            compressibleConfig: lightTokenConfig?.compressibleConfig,
            configAccount: lightTokenConfig?.configAccount,
            rentPayerPda: lightTokenConfig?.rentPayerPda,
        });
    } else {
        return createSplAssociatedTokenAccountIdempotentInstruction(
            effectivePayer,
            associatedToken,
            owner,
            mint,
            programId,
            effectiveAssociatedTokenProgramId,
        );
    }
}

export const createAta = createAssociatedTokenAccountInstruction;
export const createAtaIdempotent =
    createAssociatedTokenAccountIdempotentInstruction;

export function createAtaInstruction({
    payer,
    owner,
    mint,
    programId,
}: CreateRawAtaInstructionInput): TransactionInstruction {
    const targetProgramId = programId ?? LIGHT_TOKEN_PROGRAM_ID;
    const associatedToken = getAtaAddress({
        owner,
        mint,
        programId: targetProgramId,
    });

    return createAtaIdempotent({
        payer,
        associatedToken,
        owner,
        mint,
        programId: targetProgramId,
    });
}

export async function createAtaInstructions({
    payer,
    owner,
    mint,
    programId,
}: CreateRawAtaInstructionInput): Promise<TransactionInstruction[]> {
    return [createAtaInstruction({ payer, owner, mint, programId })];
}

export async function createAtaInstructionPlan(
    input: CreateRawAtaInstructionInput,
) {
    return toInstructionPlan(await createAtaInstructions(input));
}

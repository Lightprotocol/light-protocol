import {
    PublicKey,
    SystemProgram,
    TransactionInstruction,
} from '@solana/web3.js';
import { Buffer } from 'buffer';
import { CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { struct, u8, u32, option, vec, array } from '@coral-xyz/borsh';
import { LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR } from '../../constants';

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

export interface CreateAssociatedCTokenAccountParams {
    compressibleConfig?: CompressibleConfig;
}

/**
 * Default compressible config for c-token ATAs - matches Rust SDK defaults.
 *
 * - tokenAccountVersion: 3 (ShaFlat) - latest hashing scheme
 * - rentPayment: 16 - prepay 16 epochs (~24 hours rent)
 * - compressionOnly: 1 - required for ATAs
 * - writeTopUp: 766 - per-write top-up (~2 epochs rent) when rent < 2 epochs
 * - compressToAccountPubkey: null - required for ATAs
 *
 * Cost breakdown at ATA creation:
 * - Rent sponsor PDA (LIGHT_TOKEN_RENT_SPONSOR) pays: rent exemption (~890,880 lamports)
 * - Fee payer pays: compression_cost (11K) + 16 epochs rent (~6,400) = ~17,400 lamports + tx fees
 *
 * Per-write top-up (transfers):
 * - When account rent is below 2 epochs, fee payer pays 766 lamports top-up
 * - This keeps the account perpetually funded when actively used
 *
 * Rent calculation (272-byte compressible ctoken account):
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

function getAssociatedCTokenAddress(
    owner: PublicKey,
    mint: PublicKey,
): PublicKey {
    return PublicKey.findProgramAddressSync(
        [owner.toBuffer(), CTOKEN_PROGRAM_ID.toBuffer(), mint.toBuffer()],
        CTOKEN_PROGRAM_ID,
    )[0];
}

function encodeCreateAssociatedCTokenAccountData(
    params: CreateAssociatedCTokenAccountParams,
    idempotent: boolean,
): Buffer {
    const buffer = Buffer.alloc(2000);
    const len = CreateAssociatedTokenAccountInstructionDataLayout.encode(
        {
            compressibleConfig: params.compressibleConfig || null,
        },
        buffer,
    );

    const discriminator = idempotent
        ? CREATE_ASSOCIATED_TOKEN_ACCOUNT_IDEMPOTENT_DISCRIMINATOR
        : CREATE_ASSOCIATED_TOKEN_ACCOUNT_DISCRIMINATOR;

    return Buffer.concat([discriminator, buffer.subarray(0, len)]);
}

export interface CreateAssociatedCTokenAccountInstructionParams {
    feePayer: PublicKey;
    owner: PublicKey;
    mint: PublicKey;
    compressibleConfig?: CompressibleConfig;
    configAccount?: PublicKey;
    rentPayerPda?: PublicKey;
}

/**
 * Create instruction for creating an associated compressed token account.
 * Uses the default rent sponsor PDA by default.
 *
 * @param feePayer          Fee payer public key.
 * @param owner             Owner of the associated token account.
 * @param mint              Mint address.
 * @param compressibleConfig Compressible configuration (defaults to rent sponsor config).
 * @param configAccount     Config account (defaults to LIGHT_TOKEN_CONFIG).
 * @param rentPayerPda      Rent payer PDA (defaults to LIGHT_TOKEN_RENT_SPONSOR).
 */
// TODO: use createAssociatedCTokenAccount2.
export function createAssociatedCTokenAccountInstruction(
    feePayer: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    compressibleConfig: CompressibleConfig = DEFAULT_COMPRESSIBLE_CONFIG,
    configAccount: PublicKey = LIGHT_TOKEN_CONFIG,
    rentPayerPda: PublicKey = LIGHT_TOKEN_RENT_SPONSOR,
): TransactionInstruction {
    const associatedTokenAccount = getAssociatedCTokenAddress(owner, mint);

    const data = encodeCreateAssociatedCTokenAccountData(
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
    // 5. config account
    // 6. rent_payer PDA
    const keys = [
        { pubkey: owner, isSigner: false, isWritable: false },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: feePayer, isSigner: true, isWritable: true },
        {
            pubkey: associatedTokenAccount,
            isSigner: false,
            isWritable: true,
        },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        { pubkey: configAccount, isSigner: false, isWritable: false },
        { pubkey: rentPayerPda, isSigner: false, isWritable: true },
    ];

    return new TransactionInstruction({
        programId: CTOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

/**
 * Create idempotent instruction for creating an associated compressed token account.
 * Uses the default rent sponsor PDA by default.
 *
 * @param feePayer          Fee payer public key.
 * @param owner             Owner of the associated token account.
 * @param mint              Mint address.
 * @param compressibleConfig Compressible configuration (defaults to rent sponsor config).
 * @param configAccount     Config account (defaults to LIGHT_TOKEN_CONFIG).
 * @param rentPayerPda      Rent payer PDA (defaults to LIGHT_TOKEN_RENT_SPONSOR).
 */
export function createAssociatedCTokenAccountIdempotentInstruction(
    feePayer: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    compressibleConfig: CompressibleConfig = DEFAULT_COMPRESSIBLE_CONFIG,
    configAccount: PublicKey = LIGHT_TOKEN_CONFIG,
    rentPayerPda: PublicKey = LIGHT_TOKEN_RENT_SPONSOR,
): TransactionInstruction {
    const associatedTokenAccount = getAssociatedCTokenAddress(owner, mint);

    const data = encodeCreateAssociatedCTokenAccountData(
        {
            compressibleConfig,
        },
        true,
    );

    const keys = [
        { pubkey: owner, isSigner: false, isWritable: false },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: feePayer, isSigner: true, isWritable: true },
        {
            pubkey: associatedTokenAccount,
            isSigner: false,
            isWritable: true,
        },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        { pubkey: configAccount, isSigner: false, isWritable: false },
        { pubkey: rentPayerPda, isSigner: false, isWritable: true },
    ];

    return new TransactionInstruction({
        programId: CTOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

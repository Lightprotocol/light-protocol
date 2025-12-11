import {
    PublicKey,
    SystemProgram,
    TransactionInstruction,
} from '@solana/web3.js';
import { Buffer } from 'buffer';
import { CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { struct, u8, u32, option, vec, array } from '@coral-xyz/borsh';

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
const CompressibleExtensionInstructionDataLayout = struct([
    u8('tokenAccountVersion'),
    u8('rentPayment'),
    u8('hasTopUp'),
    u8('compressionOnly'),
    u32('writeTopUp'),
    option(CompressToPubkeyLayout, 'compressToAccountPubkey'),
]);

const CreateAssociatedTokenAccountInstructionDataLayout = struct([
    u8('bump'),
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
    hasTopUp: number;
    compressionOnly: number;
    writeTopUp: number;
    compressToAccountPubkey?: CompressToPubkey | null;
}

export interface CreateAssociatedCTokenAccountParams {
    bump: number;
    compressibleConfig?: CompressibleConfig;
}

function getAssociatedCTokenAddressAndBump(
    owner: PublicKey,
    mint: PublicKey,
): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
        [owner.toBuffer(), CTOKEN_PROGRAM_ID.toBuffer(), mint.toBuffer()],
        CTOKEN_PROGRAM_ID,
    );
}

function encodeCreateAssociatedCTokenAccountData(
    params: CreateAssociatedCTokenAccountParams,
    idempotent: boolean,
): Buffer {
    const buffer = Buffer.alloc(2000);
    const len = CreateAssociatedTokenAccountInstructionDataLayout.encode(
        {
            bump: params.bump,
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
 *
 * @param feePayer          Fee payer public key.
 * @param owner             Owner of the associated token account.
 * @param mint              Mint address.
 * @param compressibleConfig Optional compressible configuration.
 * @param configAccount     Optional config account.
 * @param rentPayerPda      Optional rent payer PDA.
 */
// TODO: use createAssociatedCTokenAccount2.
export function createAssociatedCTokenAccountInstruction(
    feePayer: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    compressibleConfig?: CompressibleConfig,
    configAccount?: PublicKey,
    rentPayerPda?: PublicKey,
): TransactionInstruction {
    const [associatedTokenAccount, bump] = getAssociatedCTokenAddressAndBump(
        owner,
        mint,
    );

    const data = encodeCreateAssociatedCTokenAccountData(
        {
            bump,
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
    // 5. optional accounts (config, rent_payer, etc.)
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
    ];

    if (compressibleConfig && configAccount && rentPayerPda) {
        keys.push(
            { pubkey: configAccount, isSigner: false, isWritable: false },
            { pubkey: rentPayerPda, isSigner: false, isWritable: true },
        );
    }

    return new TransactionInstruction({
        programId: CTOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

/**
 * Create idempotent instruction for creating an associated compressed token account.
 *
 * @param feePayer          Fee payer public key.
 * @param owner             Owner of the associated token account.
 * @param mint              Mint address.
 * @param compressibleConfig Optional compressible configuration.
 * @param configAccount     Optional config account.
 * @param rentPayerPda      Optional rent payer PDA.
 */
export function createAssociatedCTokenAccountIdempotentInstruction(
    feePayer: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    compressibleConfig?: CompressibleConfig,
    configAccount?: PublicKey,
    rentPayerPda?: PublicKey,
): TransactionInstruction {
    const [associatedTokenAccount, bump] = getAssociatedCTokenAddressAndBump(
        owner,
        mint,
    );

    const data = encodeCreateAssociatedCTokenAccountData(
        {
            bump,
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
    ];

    if (compressibleConfig && configAccount && rentPayerPda) {
        keys.push(
            { pubkey: configAccount, isSigner: false, isWritable: false },
            { pubkey: rentPayerPda, isSigner: false, isWritable: true },
        );
    }

    return new TransactionInstruction({
        programId: CTOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

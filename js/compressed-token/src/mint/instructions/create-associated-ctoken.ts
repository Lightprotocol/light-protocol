import {
    PublicKey,
    SystemProgram,
    TransactionInstruction,
} from '@solana/web3.js';
import { Buffer } from 'buffer';
import { CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import {
    ASSOCIATED_TOKEN_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    createAssociatedTokenAccountInstruction as createSplAssociatedTokenAccountInstruction,
    createAssociatedTokenAccountIdempotentInstruction as createSplAssociatedTokenAccountIdempotentInstruction,
} from '@solana/spl-token';
import { struct, u8, publicKey, option, vec } from '@coral-xyz/borsh';
import { getATAProgramId } from '../../utils';

const CREATE_ASSOCIATED_TOKEN_ACCOUNT_DISCRIMINATOR = Buffer.from([100]);
const CREATE_ASSOCIATED_TOKEN_ACCOUNT_IDEMPOTENT_DISCRIMINATOR = Buffer.from([
    102,
]);

const CompressibleExtensionInstructionDataLayout = struct([
    u8('rentPayment'),
    u8('writeTopUp'),
    option(struct([vec(u8(), 'seeds'), u8('bump')]), 'compressToAccountPubkey'),
    u8('tokenAccountVersion'),
]);

const CreateAssociatedTokenAccountInstructionDataLayout = struct([
    publicKey('owner'),
    publicKey('mint'),
    u8('bump'),
    option(CompressibleExtensionInstructionDataLayout, 'compressibleConfig'),
]);

export interface CompressibleConfig {
    rentPayment: number;
    writeTopUp: number;
    compressToAccountPubkey?: {
        seeds: number[];
        bump: number;
    };
    tokenAccountVersion: number;
}

export interface CreateAssociatedCTokenAccountParams {
    owner: PublicKey;
    mint: PublicKey;
    bump: number;
    compressibleConfig?: CompressibleConfig;
}

/**
 * CToken-specific config for createAssociatedTokenAccountInterfaceInstruction
 */
export interface CTokenConfig {
    compressibleConfig?: CompressibleConfig;
    configAccount?: PublicKey;
    rentPayerPda?: PublicKey;
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
            owner: params.owner,
            mint: params.mint,
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
            owner,
            mint,
            bump,
            compressibleConfig,
        },
        false,
    );

    const keys = [
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
            owner,
            mint,
            bump,
            compressibleConfig,
        },
        true,
    );

    const keys = [
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

// Keep old interface type for backwards compatibility export
export interface CreateAssociatedTokenAccountInterfaceInstructionParams {
    payer: PublicKey;
    associatedToken: PublicKey;
    owner: PublicKey;
    mint: PublicKey;
    programId?: PublicKey;
    associatedTokenProgramId?: PublicKey;
    compressibleConfig?: CompressibleConfig;
    configAccount?: PublicKey;
    rentPayerPda?: PublicKey;
}

/**
 * Create instruction for creating an associated token account (SPL, Token-2022, or CToken).
 * Follows SPL Token API signature with optional CToken config at the end.
 *
 * @param payer                    Fee payer public key.
 * @param associatedToken          Associated token account address.
 * @param owner                    Owner of the associated token account.
 * @param mint                     Mint address.
 * @param programId                Token program ID (default: TOKEN_PROGRAM_ID).
 * @param associatedTokenProgramId Associated token program ID.
 * @param ctokenConfig             Optional CToken-specific configuration.
 */
export function createAssociatedTokenAccountInterfaceInstruction(
    payer: PublicKey,
    associatedToken: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    programId: PublicKey = TOKEN_PROGRAM_ID,
    associatedTokenProgramId?: PublicKey,
    ctokenConfig?: CTokenConfig,
): TransactionInstruction {
    const effectiveAssociatedTokenProgramId =
        associatedTokenProgramId ?? getATAProgramId(programId);

    if (programId.equals(CTOKEN_PROGRAM_ID)) {
        return createAssociatedCTokenAccountInstruction(
            payer,
            owner,
            mint,
            ctokenConfig?.compressibleConfig,
            ctokenConfig?.configAccount,
            ctokenConfig?.rentPayerPda,
        );
    } else {
        return createSplAssociatedTokenAccountInstruction(
            payer,
            associatedToken,
            owner,
            mint,
            programId,
            effectiveAssociatedTokenProgramId,
        );
    }
}

/**
 * Create idempotent instruction for creating an associated token account (SPL, Token-2022, or CToken).
 * Follows SPL Token API signature with optional CToken config at the end.
 *
 * @param payer                    Fee payer public key.
 * @param associatedToken          Associated token account address.
 * @param owner                    Owner of the associated token account.
 * @param mint                     Mint address.
 * @param programId                Token program ID (default: TOKEN_PROGRAM_ID).
 * @param associatedTokenProgramId Associated token program ID.
 * @param ctokenConfig             Optional CToken-specific configuration.
 */
export function createAssociatedTokenAccountInterfaceIdempotentInstruction(
    payer: PublicKey,
    associatedToken: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    programId: PublicKey = TOKEN_PROGRAM_ID,
    associatedTokenProgramId?: PublicKey,
    ctokenConfig?: CTokenConfig,
): TransactionInstruction {
    const effectiveAssociatedTokenProgramId =
        associatedTokenProgramId ?? getATAProgramId(programId);

    if (programId.equals(CTOKEN_PROGRAM_ID)) {
        return createAssociatedCTokenAccountIdempotentInstruction(
            payer,
            owner,
            mint,
            ctokenConfig?.compressibleConfig,
            ctokenConfig?.configAccount,
            ctokenConfig?.rentPayerPda,
        );
    } else {
        return createSplAssociatedTokenAccountIdempotentInstruction(
            payer,
            associatedToken,
            owner,
            mint,
            programId,
            effectiveAssociatedTokenProgramId,
        );
    }
}

/**
 * Short alias for createAssociatedTokenAccountInterfaceIdempotentInstruction.
 */
export const createATAInterfaceIdempotentInstruction =
    createAssociatedTokenAccountInterfaceIdempotentInstruction;

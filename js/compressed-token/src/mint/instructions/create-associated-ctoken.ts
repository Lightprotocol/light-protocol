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
} from '@solana/spl-token';
import { struct, u8, publicKey, option, vec } from '@coral-xyz/borsh';

const CREATE_ASSOCIATED_TOKEN_ACCOUNT_DISCRIMINATOR = Buffer.from([103]);
const CREATE_ASSOCIATED_TOKEN_ACCOUNT_IDEMPOTENT_DISCRIMINATOR = Buffer.from([
    105,
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

export function createAssociatedTokenAccountInterfaceInstruction(
    payer: PublicKey,
    associatedToken: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    programId: PublicKey = TOKEN_PROGRAM_ID,
    associatedTokenProgramId?: PublicKey,
    compressibleConfig?: CompressibleConfig,
    configAccount?: PublicKey,
    rentPayerPda?: PublicKey,
): TransactionInstruction {
    const effectiveAssociatedTokenProgramId =
        associatedTokenProgramId ??
        (programId.equals(CTOKEN_PROGRAM_ID)
            ? CTOKEN_PROGRAM_ID
            : ASSOCIATED_TOKEN_PROGRAM_ID);

    console.log(
        'createAssociatedTokenAccountInterfaceInstruction',
        programId,
        associatedTokenProgramId,
    );

    if (programId.equals(CTOKEN_PROGRAM_ID)) {
        return createAssociatedCTokenAccountInstruction(
            payer,
            owner,
            mint,
            compressibleConfig,
            configAccount,
            rentPayerPda,
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

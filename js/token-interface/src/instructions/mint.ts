import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
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
    CreateRawMintInstructionInput,
    CreateRawMintToInstructionInput,
} from '../types';
import { toBigIntAmount } from '../helpers';
import { getMint } from '../read/get-mint';
import { createSplInterfaceInstruction } from './spl-interface';
import { toInstructionPlan } from './_plan';

const LIGHT_TOKEN_MINT_TO_DISCRIMINATOR = 7;

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

/**
 * Build canonical mint creation flow for SPL/T22 + SPL interface index 0.
 * Order must stay: create account -> initialize mint -> create SPL interface.
 */
export async function createMintInstructions({
    rpc,
    payer,
    mint,
    decimals,
    mintAuthority,
    freezeAuthority = null,
    tokenProgramId = TOKEN_PROGRAM_ID,
    mintSize = MINT_SIZE,
    rentExemptBalance,
    splInterfaceIndex = 0,
}: CreateMintInstructionsInput): Promise<TransactionInstruction[]> {
    assertSupportedMintProgram(tokenProgramId);

    const lamports =
        rentExemptBalance ??
        (await rpc.getMinimumBalanceForRentExemption(mintSize));

    const createMintAccountInstruction = SystemProgram.createAccount({
        fromPubkey: payer,
        lamports,
        newAccountPubkey: mint,
        programId: tokenProgramId,
        space: mintSize,
    });

    return [
        createMintAccountInstruction,
        createMintInstruction({
            mint,
            decimals,
            mintAuthority,
            freezeAuthority,
            tokenProgramId,
        }),
        createSplInterfaceInstruction({
            feePayer: payer,
            mint,
            index: splInterfaceIndex,
            tokenProgramId,
        }),
    ];
}

/**
 * Create mint-to instruction using SPL/T22 or light-token semantics.
 *
 * SPL/T22 path mirrors spl-token mintTo.
 * light-token path mirrors v3 mintToInterface behavior.
 */
export function createMintToInstruction({
    mint,
    destination,
    authority,
    amount,
    payer,
    tokenProgramId = LIGHT_TOKEN_PROGRAM_ID,
    multiSigners = [],
    maxTopUp,
}: CreateRawMintToInstructionInput): TransactionInstruction {
    const amountBigInt = toBigIntAmount(amount);

    if (
        tokenProgramId.equals(TOKEN_PROGRAM_ID) ||
        tokenProgramId.equals(TOKEN_2022_PROGRAM_ID)
    ) {
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
 * Resolve mint program from chain and build one mint-to instruction.
 */
export async function createMintToInstructions({
    rpc,
    mint,
    destination,
    authority,
    amount,
    payer,
    tokenProgramId,
    multiSigners,
    maxTopUp,
}: CreateMintToInstructionsInput): Promise<TransactionInstruction[]> {
    const mintInfo = await getMint(rpc, mint, undefined, tokenProgramId);

    return [
        createMintToInstruction({
            mint,
            destination,
            authority,
            amount,
            payer,
            tokenProgramId: mintInfo.programId,
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

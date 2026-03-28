import { TransactionInstruction } from '@solana/web3.js';
import { Buffer } from 'buffer';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import type {
    CreateFreezeInstructionsInput,
    CreateRawFreezeInstructionInput,
} from '../types';
import { getAtaAddress } from '../read';
import { createLoadInstructions } from './load';
import { toInstructionPlan } from './_plan';

const LIGHT_TOKEN_FREEZE_ACCOUNT_DISCRIMINATOR = Buffer.from([10]);

export function createFreezeInstruction({
    tokenAccount,
    mint,
    freezeAuthority,
}: CreateRawFreezeInstructionInput): TransactionInstruction {
    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys: [
            { pubkey: tokenAccount, isSigner: false, isWritable: true },
            { pubkey: mint, isSigner: false, isWritable: false },
            { pubkey: freezeAuthority, isSigner: true, isWritable: false },
        ],
        data: LIGHT_TOKEN_FREEZE_ACCOUNT_DISCRIMINATOR,
    });
}

export async function createFreezeInstructions({
    rpc,
    payer,
    owner,
    mint,
    freezeAuthority,
}: CreateFreezeInstructionsInput): Promise<TransactionInstruction[]> {
    const tokenAccount = getAtaAddress({ owner, mint });

    return [
        ...(await createLoadInstructions({
            rpc,
            payer,
            owner,
            mint,
            wrap: true,
        })),
        createFreezeInstruction({
            tokenAccount,
            mint,
            freezeAuthority,
        }),
    ];
}

export async function createFreezeInstructionsNowrap({
    rpc,
    payer,
    owner,
    mint,
    freezeAuthority,
}: CreateFreezeInstructionsInput): Promise<TransactionInstruction[]> {
    const tokenAccount = getAtaAddress({ owner, mint });

    return [
        ...(await createLoadInstructions({
            rpc,
            payer,
            owner,
            mint,
            wrap: false,
        })),
        createFreezeInstruction({
            tokenAccount,
            mint,
            freezeAuthority,
        }),
    ];
}

export async function createFreezeInstructionPlan(
    input: CreateFreezeInstructionsInput,
) {
    return toInstructionPlan(await createFreezeInstructions(input));
}

import { TransactionInstruction } from '@solana/web3.js';
import { Buffer } from 'buffer';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import {
    assertAccountNotFrozen,
    getAta,
} from '../account';
import type {
    CreateFreezeInstructionsInput,
    CreateRawFreezeInstructionInput,
} from '../types';
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
    const account = await getAta({ rpc, owner, mint });

    assertAccountNotFrozen(account, 'freeze');

    return [
        ...(await createLoadInstructions({
            rpc,
            payer,
            owner,
            mint,
            account,
            wrap: true,
        })),
        createFreezeInstruction({
            tokenAccount: account.address,
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
    const account = await getAta({ rpc, owner, mint });

    assertAccountNotFrozen(account, 'freeze');

    return [
        ...(await createLoadInstructions({
            rpc,
            payer,
            owner,
            mint,
            account,
            wrap: false,
        })),
        createFreezeInstruction({
            tokenAccount: account.address,
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

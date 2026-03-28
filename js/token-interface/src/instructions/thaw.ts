import { TransactionInstruction } from '@solana/web3.js';
import { Buffer } from 'buffer';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import {
    assertAccountFrozen,
    getAta,
} from '../account';
import type {
    CreateRawThawInstructionInput,
    CreateThawInstructionsInput,
} from '../types';
import { createLoadInstructions } from './load';
import { toInstructionPlan } from './_plan';

const LIGHT_TOKEN_THAW_ACCOUNT_DISCRIMINATOR = Buffer.from([11]);

export function createThawInstruction({
    tokenAccount,
    mint,
    freezeAuthority,
}: CreateRawThawInstructionInput): TransactionInstruction {
    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys: [
            { pubkey: tokenAccount, isSigner: false, isWritable: true },
            { pubkey: mint, isSigner: false, isWritable: false },
            { pubkey: freezeAuthority, isSigner: true, isWritable: false },
        ],
        data: LIGHT_TOKEN_THAW_ACCOUNT_DISCRIMINATOR,
    });
}

export async function createThawInstructions({
    rpc,
    payer,
    owner,
    mint,
    freezeAuthority,
}: CreateThawInstructionsInput): Promise<TransactionInstruction[]> {
    const account = await getAta({ rpc, owner, mint });

    assertAccountFrozen(account, 'thaw');

    return [
        ...(await createLoadInstructions({
            rpc,
            payer,
            owner,
            mint,
            wrap: true,
        })),
        createThawInstruction({
            tokenAccount: account.address,
            mint,
            freezeAuthority,
        }),
    ];
}

export async function createThawInstructionsNowrap({
    rpc,
    payer,
    owner,
    mint,
    freezeAuthority,
}: CreateThawInstructionsInput): Promise<TransactionInstruction[]> {
    const account = await getAta({ rpc, owner, mint });

    assertAccountFrozen(account, 'thaw');

    return [
        ...(await createLoadInstructions({
            rpc,
            payer,
            owner,
            mint,
            wrap: false,
        })),
        createThawInstruction({
            tokenAccount: account.address,
            mint,
            freezeAuthority,
        }),
    ];
}

export async function createThawInstructionPlan(
    input: CreateThawInstructionsInput,
) {
    return toInstructionPlan(await createThawInstructions(input));
}

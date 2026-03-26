import { SystemProgram, TransactionInstruction } from '@solana/web3.js';
import { Buffer } from 'buffer';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { assertAccountNotFrozen, getAta } from '../account';
import type {
    CreateRawRevokeInstructionInput,
    CreateRevokeInstructionsInput,
} from '../types';
import { buildLoadInstructionList } from './load';
import { toInstructionPlan } from './_plan';

const LIGHT_TOKEN_REVOKE_DISCRIMINATOR = 5;

export function createRevokeInstruction({
    tokenAccount,
    owner,
    payer,
}: CreateRawRevokeInstructionInput): TransactionInstruction {
    const effectiveFeePayer = payer ?? owner;

    const keys = [
        { pubkey: tokenAccount, isSigner: false, isWritable: true },
        {
            pubkey: owner,
            isSigner: true,
            isWritable: effectiveFeePayer.equals(owner),
        },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        {
            pubkey: effectiveFeePayer,
            isSigner: !effectiveFeePayer.equals(owner),
            isWritable: true,
        },
    ];

    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys,
        data: Buffer.from([LIGHT_TOKEN_REVOKE_DISCRIMINATOR]),
    });
}

export async function createRevokeInstructions({
    rpc,
    payer,
    owner,
    mint,
}: CreateRevokeInstructionsInput): Promise<TransactionInstruction[]> {
    const account = await getAta({
        rpc,
        owner,
        mint,
    });

    assertAccountNotFrozen(account, 'revoke');

    return [
        ...(await buildLoadInstructionList({
            rpc,
            payer,
            owner,
            mint,
            account,
            wrap: true,
        })),
        createRevokeInstruction({
            tokenAccount: account.address,
            owner,
            payer,
        }),
    ];
}

export async function createRevokeInstructionsNowrap({
    rpc,
    payer,
    owner,
    mint,
}: CreateRevokeInstructionsInput): Promise<TransactionInstruction[]> {
    const account = await getAta({
        rpc,
        owner,
        mint,
    });

    return [
        ...(await buildLoadInstructionList({
            rpc,
            payer,
            owner,
            mint,
            account,
            wrap: false,
        })),
        createRevokeInstruction({
            tokenAccount: account.address,
            owner,
            payer,
        }),
    ];
}

export async function createRevokeInstructionPlan(
    input: CreateRevokeInstructionsInput,
) {
    return toInstructionPlan(await createRevokeInstructions(input));
}

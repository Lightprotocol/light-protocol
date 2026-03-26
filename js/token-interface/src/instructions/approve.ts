import { SystemProgram, TransactionInstruction } from '@solana/web3.js';
import { Buffer } from 'buffer';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { assertAccountNotFrozen, getAta } from '../account';
import type {
    CreateApproveInstructionsInput,
    CreateRawApproveInstructionInput,
} from '../types';
import { buildLoadInstructionList } from './load';
import { toInstructionPlan } from './_plan';

const LIGHT_TOKEN_APPROVE_DISCRIMINATOR = 4;

function toBigIntAmount(amount: number | bigint): bigint {
    return BigInt(amount.toString());
}

export function createApproveInstruction({
    tokenAccount,
    delegate,
    owner,
    amount,
    payer,
}: CreateRawApproveInstructionInput): TransactionInstruction {
    const data = Buffer.alloc(9);
    data.writeUInt8(LIGHT_TOKEN_APPROVE_DISCRIMINATOR, 0);
    data.writeBigUInt64LE(BigInt(amount), 1);

    const effectiveFeePayer = payer ?? owner;

    const keys = [
        { pubkey: tokenAccount, isSigner: false, isWritable: true },
        { pubkey: delegate, isSigner: false, isWritable: false },
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
        data,
    });
}

export async function createApproveInstructions({
    rpc,
    payer,
    owner,
    mint,
    delegate,
    amount,
}: CreateApproveInstructionsInput): Promise<TransactionInstruction[]> {
    const account = await getAta({
        rpc,
        owner,
        mint,
    });

    assertAccountNotFrozen(account, 'approve');

    return [
        ...(await buildLoadInstructionList({
            rpc,
            payer,
            owner,
            mint,
            account,
            wrap: true,
        })),
        createApproveInstruction({
            tokenAccount: account.address,
            delegate,
            owner,
            amount: toBigIntAmount(amount),
            payer,
        }),
    ];
}

export async function createApproveInstructionsNowrap({
    rpc,
    payer,
    owner,
    mint,
    delegate,
    amount,
}: CreateApproveInstructionsInput): Promise<TransactionInstruction[]> {
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
        createApproveInstruction({
            tokenAccount: account.address,
            delegate,
            owner,
            amount: toBigIntAmount(amount),
            payer,
        }),
    ];
}

export async function createApproveInstructionPlan(
    input: CreateApproveInstructionsInput,
) {
    return toInstructionPlan(await createApproveInstructions(input));
}

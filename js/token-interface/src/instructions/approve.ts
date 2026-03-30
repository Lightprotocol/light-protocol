import { SystemProgram, TransactionInstruction } from '@solana/web3.js';
import { Buffer } from 'buffer';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import type {
    CreateApproveInstructionsInput,
    CreateRawApproveInstructionInput,
} from '../types';
import { toBigIntAmount } from '../helpers';
import { getAtaAddress } from '../read';
import { createLoadInstructions } from './load';
import { toInstructionPlan } from './_plan';

const LIGHT_TOKEN_APPROVE_DISCRIMINATOR = 4;

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
    return _createApproveInstructions(
        { rpc, payer, owner, mint, delegate, amount },
        true,
    );
}

/**
 * @internal
 */
async function _createApproveInstructions(
    {
        rpc,
        payer,
        owner,
        mint,
        delegate,
        amount,
    }: CreateApproveInstructionsInput,
    wrap: boolean,
): Promise<TransactionInstruction[]> {
    const tokenAccount = getAtaAddress({ owner, mint });

    return [
        ...(await createLoadInstructions({
            rpc,
            payer,
            owner,
            mint,
            wrap,
        })),
        createApproveInstruction({
            tokenAccount,
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

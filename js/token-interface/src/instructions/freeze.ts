import { TransactionInstruction } from '@solana/web3.js';
import { Buffer } from 'buffer';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import type {
    CreateFreezeInstructionsInput,
    CreateRawFreezeInstructionInput,
} from '../types';
import { getAssociatedTokenAddress } from '../read';
import { createLoadInstructions } from './load';
import { toInstructionPlan } from './_plan';

const LIGHT_TOKEN_FREEZE_ACCOUNT_DISCRIMINATOR = 10;

export function createFreezeInstruction({
    tokenAccount,
    mint,
    freezeAuthority,
}: CreateRawFreezeInstructionInput): TransactionInstruction {
    const data = Buffer.alloc(1);
    data.writeUInt8(LIGHT_TOKEN_FREEZE_ACCOUNT_DISCRIMINATOR, 0);

    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys: [
            { pubkey: tokenAccount, isSigner: false, isWritable: true },
            { pubkey: mint, isSigner: false, isWritable: false },
            { pubkey: freezeAuthority, isSigner: true, isWritable: false },
        ],
        data,
    });
}

export async function createFreezeInstructions({
    rpc,
    payer,
    owner,
    mint,
    freezeAuthority,
}: CreateFreezeInstructionsInput): Promise<TransactionInstruction[]> {
    return _createFreezeInstructions(
        { rpc, payer, owner, mint, freezeAuthority },
        true,
    );
}

/**
 * @internal
 */
async function _createFreezeInstructions(
    { rpc, payer, owner, mint, freezeAuthority }: CreateFreezeInstructionsInput,
    wrap: boolean,
): Promise<TransactionInstruction[]> {
    const tokenAccount = getAssociatedTokenAddress(mint, owner, true);

    return [
        ...(await createLoadInstructions({
            rpc,
            payer,
            owner,
            mint,
            wrap,
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

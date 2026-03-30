import { SystemProgram, TransactionInstruction } from '@solana/web3.js';
import { Buffer } from 'buffer';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import type {
    CreateRawRevokeInstructionInput,
    CreateRevokeInstructionsInput,
} from '../types';
import { getAtaAddress } from '../read';
import { createLoadInstructions } from './load';
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
    return _createRevokeInstructions({ rpc, payer, owner, mint }, true);
}

/**
 * @internal
 */
async function _createRevokeInstructions(
    { rpc, payer, owner, mint }: CreateRevokeInstructionsInput,
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
        createRevokeInstruction({
            tokenAccount,
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

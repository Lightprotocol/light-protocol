import { TransactionInstruction } from '@solana/web3.js';
import { Buffer } from 'buffer';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import type {
    CreateRawThawInstructionInput,
    CreateThawInstructionsInput,
} from '../types';
import { getAtaAddress } from '../read';
import { createLoadInstructions } from './load';
import { toInstructionPlan } from './_plan';

const LIGHT_TOKEN_THAW_ACCOUNT_DISCRIMINATOR = 11;

export function createThawInstruction({
    tokenAccount,
    mint,
    freezeAuthority,
}: CreateRawThawInstructionInput): TransactionInstruction {
    const data = Buffer.alloc(1);
    data.writeUInt8(LIGHT_TOKEN_THAW_ACCOUNT_DISCRIMINATOR, 0);

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

export async function createThawInstructions({
    rpc,
    payer,
    owner,
    mint,
    freezeAuthority,
}: CreateThawInstructionsInput): Promise<TransactionInstruction[]> {
    return _createThawInstructions(
        { rpc, payer, owner, mint, freezeAuthority },
        true,
    );
}

/**
 * @internal
 */
async function _createThawInstructions(
    { rpc, payer, owner, mint, freezeAuthority }: CreateThawInstructionsInput,
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
            allowFrozen: true,
        })),
        createThawInstruction({
            tokenAccount,
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

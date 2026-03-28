import { Buffer } from 'buffer';
import { SystemProgram, TransactionInstruction } from '@solana/web3.js';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import type {
    CreateBurnInstructionsInput,
    CreateRawBurnCheckedInstructionInput,
    CreateRawBurnInstructionInput,
} from '../types';
import { getAtaAddress } from '../read';
import { createLoadInstructions } from './load';
import { toInstructionPlan } from './_plan';

const LIGHT_TOKEN_BURN_DISCRIMINATOR = 8;
const LIGHT_TOKEN_BURN_CHECKED_DISCRIMINATOR = 15;

function toBigIntAmount(amount: number | bigint): bigint {
    return BigInt(amount.toString());
}

export function createBurnInstruction({
    source,
    mint,
    authority,
    amount,
    payer,
}: CreateRawBurnInstructionInput): TransactionInstruction {
    const data = Buffer.alloc(9);
    data.writeUInt8(LIGHT_TOKEN_BURN_DISCRIMINATOR, 0);
    data.writeBigUInt64LE(BigInt(amount), 1);

    const effectivePayer = payer ?? authority;

    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys: [
            { pubkey: source, isSigner: false, isWritable: true },
            { pubkey: mint, isSigner: false, isWritable: true },
            {
                pubkey: authority,
                isSigner: true,
                isWritable: effectivePayer.equals(authority),
            },
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            {
                pubkey: effectivePayer,
                isSigner: !effectivePayer.equals(authority),
                isWritable: true,
            },
        ],
        data,
    });
}

export function createBurnCheckedInstruction({
    source,
    mint,
    authority,
    amount,
    decimals,
    payer,
}: CreateRawBurnCheckedInstructionInput): TransactionInstruction {
    const data = Buffer.alloc(10);
    data.writeUInt8(LIGHT_TOKEN_BURN_CHECKED_DISCRIMINATOR, 0);
    data.writeBigUInt64LE(BigInt(amount), 1);
    data.writeUInt8(decimals, 9);

    const effectivePayer = payer ?? authority;

    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys: [
            { pubkey: source, isSigner: false, isWritable: true },
            { pubkey: mint, isSigner: false, isWritable: true },
            {
                pubkey: authority,
                isSigner: true,
                isWritable: effectivePayer.equals(authority),
            },
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            {
                pubkey: effectivePayer,
                isSigner: !effectivePayer.equals(authority),
                isWritable: true,
            },
        ],
        data,
    });
}

export async function createBurnInstructions({
    rpc,
    payer,
    owner,
    mint,
    authority,
    amount,
    decimals,
}: CreateBurnInstructionsInput): Promise<TransactionInstruction[]> {
    const tokenAccount = getAtaAddress({ owner, mint });

    const amountBn = toBigIntAmount(amount);
    const burnIx =
        decimals !== undefined
            ? createBurnCheckedInstruction({
                  source: tokenAccount,
                  mint,
                  authority,
                  amount: amountBn,
                  decimals,
                  payer,
              })
            : createBurnInstruction({
                  source: tokenAccount,
                  mint,
                  authority,
                  amount: amountBn,
                  payer,
              });

    return [
        ...(await createLoadInstructions({
            rpc,
            payer,
            owner,
            mint,
            authority,
            wrap: true,
        })),
        burnIx,
    ];
}

export async function createBurnInstructionsNowrap({
    rpc,
    payer,
    owner,
    mint,
    authority,
    amount,
    decimals,
}: CreateBurnInstructionsInput): Promise<TransactionInstruction[]> {
    const tokenAccount = getAtaAddress({ owner, mint });

    const amountBn = toBigIntAmount(amount);
    const burnIx =
        decimals !== undefined
            ? createBurnCheckedInstruction({
                  source: tokenAccount,
                  mint,
                  authority,
                  amount: amountBn,
                  decimals,
                  payer,
              })
            : createBurnInstruction({
                  source: tokenAccount,
                  mint,
                  authority,
                  amount: amountBn,
                  payer,
              });

    return [
        ...(await createLoadInstructions({
            rpc,
            payer,
            owner,
            mint,
            authority,
            wrap: false,
        })),
        burnIx,
    ];
}

export async function createBurnInstructionPlan(
    input: CreateBurnInstructionsInput,
) {
    return toInstructionPlan(await createBurnInstructions(input));
}

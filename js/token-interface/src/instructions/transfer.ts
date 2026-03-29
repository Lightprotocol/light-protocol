import { Buffer } from 'buffer';
import { SystemProgram, TransactionInstruction } from '@solana/web3.js';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { getSplInterfaces } from '../spl-interface';
import { createUnwrapInstruction } from './unwrap';
import { getMintDecimals, toBigIntAmount } from '../helpers';
import { getAtaAddress } from '../read';
import type {
    CreateRawTransferInstructionInput,
    CreateTransferInstructionsInput,
} from '../types';
import { createLoadInstructions } from './load';
import { toInstructionPlan } from './_plan';
import { createAtaInstruction } from './ata';

const LIGHT_TOKEN_TRANSFER_CHECKED_DISCRIMINATOR = 12;

export function createTransferCheckedInstruction({
    source,
    destination,
    mint,
    authority,
    payer,
    amount,
    decimals,
}: CreateRawTransferInstructionInput): TransactionInstruction {
    const data = Buffer.alloc(10);
    data.writeUInt8(LIGHT_TOKEN_TRANSFER_CHECKED_DISCRIMINATOR, 0);
    data.writeBigUInt64LE(BigInt(amount), 1);
    data.writeUInt8(decimals, 9);

    const effectivePayer = payer ?? authority;

    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys: [
            { pubkey: source, isSigner: false, isWritable: true },
            { pubkey: mint, isSigner: false, isWritable: false },
            { pubkey: destination, isSigner: false, isWritable: true },
            {
                pubkey: authority,
                isSigner: true,
                isWritable: effectivePayer.equals(authority),
            },
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: effectivePayer,
                isSigner: !effectivePayer.equals(authority),
                isWritable: true,
            },
        ],
        data,
    });
}

/**
 * Canonical web3.js transfer flow builder.
 * Returns an instruction array for a single transfer flow (setup + transfer).
 */
export async function createTransferInstructions({
    rpc,
    payer,
    mint,
    sourceOwner,
    authority,
    recipient,
    tokenProgram,
    amount,
}: CreateTransferInstructionsInput): Promise<TransactionInstruction[]> {
    const effectivePayer = payer ?? authority;
    const amountBigInt = toBigIntAmount(amount);
    const recipientTokenProgramId = tokenProgram ?? LIGHT_TOKEN_PROGRAM_ID;
    const [decimals, transferSplInterfaces] = await Promise.all([
        getMintDecimals(rpc, mint),
        recipientTokenProgramId.equals(LIGHT_TOKEN_PROGRAM_ID)
            ? Promise.resolve(undefined)
            : getSplInterfaces(rpc, mint),
    ]);
    const senderLoadInstructions = await createLoadInstructions({
        rpc,
        payer: effectivePayer,
        owner: sourceOwner,
        mint,
        authority,
        wrap: true,
        decimals,
        splInterfaces: transferSplInterfaces,
    });
    const recipientAta = getAtaAddress({
        owner: recipient,
        mint,
        programId: recipientTokenProgramId,
    });
    const recipientLoadInstructions: TransactionInstruction[] = [];
    const senderAta = getAtaAddress({
        owner: sourceOwner,
        mint,
    });
    let transferInstruction: TransactionInstruction;
    if (recipientTokenProgramId.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        transferInstruction = createTransferCheckedInstruction({
            source: senderAta,
            destination: recipientAta,
            mint,
            authority,
            payer: effectivePayer,
            amount: amountBigInt,
            decimals,
        });
    } else {
        if (!transferSplInterfaces) {
            throw new Error(
                'Missing SPL interfaces for non-light transfer path.',
            );
        }
        const splInterface = transferSplInterfaces.find(
            info =>
                info.isInitialized &&
                info.tokenProgramId.equals(recipientTokenProgramId),
        );
        if (!splInterface) {
            throw new Error(
                `No initialized SPL pool found for tokenProgram ${recipientTokenProgramId.toBase58()}.`,
            );
        }
        transferInstruction = createUnwrapInstruction({
            source: senderAta,
            destination: recipientAta,
            owner: authority,
            mint,
            amount: amountBigInt,
            splInterface,
            decimals,
            payer: effectivePayer,
        });
    }

    return [
        ...senderLoadInstructions,
        createAtaInstruction({
            payer: effectivePayer,
            owner: recipient,
            mint,
            programId: recipientTokenProgramId,
        }),
        ...recipientLoadInstructions,
        transferInstruction,
    ];
}

/**
 * No-wrap transfer flow builder (advanced).
 */
export async function createTransferInstructionsNowrap({
    rpc,
    payer,
    mint,
    sourceOwner,
    authority,
    recipient,
    tokenProgram,
    amount,
}: CreateTransferInstructionsInput): Promise<TransactionInstruction[]> {
    const effectivePayer = payer ?? authority;
    const amountBigInt = toBigIntAmount(amount);
    const recipientTokenProgramId = tokenProgram ?? LIGHT_TOKEN_PROGRAM_ID;
    const [decimals, transferSplInterfaces] = await Promise.all([
        getMintDecimals(rpc, mint),
        recipientTokenProgramId.equals(LIGHT_TOKEN_PROGRAM_ID)
            ? Promise.resolve(undefined)
            : getSplInterfaces(rpc, mint),
    ]);
    const senderLoadInstructions = await createLoadInstructions({
        rpc,
        payer: effectivePayer,
        owner: sourceOwner,
        mint,
        authority,
        wrap: false,
        decimals,
        splInterfaces: transferSplInterfaces,
    });
    const recipientAta = getAtaAddress({
        owner: recipient,
        mint,
        programId: recipientTokenProgramId,
    });
    const senderAta = getAtaAddress({
        owner: sourceOwner,
        mint,
    });

    let transferInstruction: TransactionInstruction;
    if (recipientTokenProgramId.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        transferInstruction = createTransferCheckedInstruction({
            source: senderAta,
            destination: recipientAta,
            mint,
            authority,
            payer: effectivePayer,
            amount: amountBigInt,
            decimals,
        });
    } else {
        if (!transferSplInterfaces) {
            throw new Error(
                'Missing SPL interfaces for non-light transfer path.',
            );
        }
        const splInterface = transferSplInterfaces.find(
            info =>
                info.isInitialized &&
                info.tokenProgramId.equals(recipientTokenProgramId),
        );
        if (!splInterface) {
            throw new Error(
                `No initialized SPL pool found for tokenProgram ${recipientTokenProgramId.toBase58()}.`,
            );
        }
        transferInstruction = createUnwrapInstruction({
            source: senderAta,
            destination: recipientAta,
            owner: authority,
            mint,
            amount: amountBigInt,
            splInterface,
            decimals,
            payer: effectivePayer,
        });
    }

    return [...senderLoadInstructions, transferInstruction];
}

export async function createTransferInstructionPlan(
    input: CreateTransferInstructionsInput,
) {
    return toInstructionPlan(await createTransferInstructions(input));
}

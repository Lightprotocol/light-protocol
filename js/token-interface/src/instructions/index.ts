import type { TransactionInstruction } from '@solana/web3.js';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import {
    createUnwrapInstruction,
    getSplInterfaceInfos,
} from '@lightprotocol/compressed-token';
import {
    TOKEN_2022_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    createCloseAccountInstruction,
    unpackAccount,
} from '@solana/spl-token';
import { assertAccountNotFrozen, getAta } from '../account';
import { getMintDecimals } from '../helpers';
import { createLoadInstructionInternal } from '../load';
import { getAtaAddress } from '../read';
import type {
    CreateApproveInstructionsInput,
    CreateAtaInstructionsInput,
    CreateFreezeInstructionsInput,
    CreateLoadInstructionsInput,
    CreateRevokeInstructionsInput,
    CreateThawInstructionsInput,
    CreateTransferInstructionsInput,
} from '../types';
import {
    createApproveInstruction,
    createAtaInstruction,
    createFreezeInstruction,
    createRevokeInstruction,
    createThawInstruction,
    createTransferCheckedInstruction,
} from './raw';

/*
 * Canonical async instruction builders: transfer, approve, and revoke prepend sender-side load
 * instructions when cold storage must be decompressed first. createAtaInstructions only emits the
 * ATA creation instruction. createFreezeInstructions and createThawInstructions do not load.
 * For manual load-only composition, see createLoadInstructions.
 */

const ZERO = BigInt(0);

function toBigIntAmount(amount: number | bigint): bigint {
    return BigInt(amount.toString());
}

async function buildLoadInstructions(
    input: CreateLoadInstructionsInput & {
        authority?: CreateTransferInstructionsInput['authority'];
        account?: Awaited<ReturnType<typeof getAta>>;
        wrap?: boolean;
    },
): Promise<TransactionInstruction[]> {
    const load = await createLoadInstructionInternal(input);

    if (!load) {
        return [];
    }

    return load.instructions;
}

async function getDerivedAtaBalance(
    rpc: CreateTransferInstructionsInput['rpc'],
    owner: CreateTransferInstructionsInput['sourceOwner'],
    mint: CreateTransferInstructionsInput['mint'],
    programId: typeof TOKEN_PROGRAM_ID | typeof TOKEN_2022_PROGRAM_ID,
): Promise<bigint> {
    const ata = getAtaAddress({ owner, mint, programId });
    const info = await rpc.getAccountInfo(ata);
    if (!info || !info.owner.equals(programId)) {
        return ZERO;
    }

    return unpackAccount(ata, info, programId).amount;
}

export async function createAtaInstructions({
    payer,
    owner,
    mint,
    programId,
}: CreateAtaInstructionsInput): Promise<TransactionInstruction[]> {
    return [createAtaInstruction({ payer, owner, mint, programId })];
}

/**
 * Advanced: standalone load (decompress) instructions for an ATA, plus create-ATA if missing.
 * Prefer the canonical builders (`buildTransferInstructions`, `createApproveInstructions`,
 * `createRevokeInstructions`, …), which prepend load automatically when needed.
 */
export async function createLoadInstructions({
    rpc,
    payer,
    owner,
    mint,
}: CreateLoadInstructionsInput): Promise<TransactionInstruction[]> {
    return buildLoadInstructions({
        rpc,
        payer,
        owner,
        mint,
        wrap: true,
    });
}

/**
 * Canonical web3.js transfer flow builder.
 * Returns an instruction array for a single transfer flow (setup + transfer).
 */
export async function buildTransferInstructions({
    rpc,
    payer,
    mint,
    sourceOwner,
    authority,
    recipient,
    tokenProgram,
    amount,
}: CreateTransferInstructionsInput): Promise<TransactionInstruction[]> {
    const amountBigInt = toBigIntAmount(amount);
    const senderLoadInstructions = await buildLoadInstructions({
        rpc,
        payer,
        owner: sourceOwner,
        mint,
        authority,
        wrap: true,
    });
    const recipientTokenProgramId = tokenProgram ?? LIGHT_TOKEN_PROGRAM_ID;
    const recipientAta = getAtaAddress({
        owner: recipient,
        mint,
        programId: recipientTokenProgramId,
    });
    const decimals = await getMintDecimals(rpc, mint);
    const [senderSplBalance, senderT22Balance] = await Promise.all([
        getDerivedAtaBalance(rpc, sourceOwner, mint, TOKEN_PROGRAM_ID),
        getDerivedAtaBalance(rpc, sourceOwner, mint, TOKEN_2022_PROGRAM_ID),
    ]);

    const closeWrappedSourceInstructions: TransactionInstruction[] = [];
    if (authority.equals(sourceOwner) && senderSplBalance > ZERO) {
        closeWrappedSourceInstructions.push(
            createCloseAccountInstruction(
                getAtaAddress({
                    owner: sourceOwner,
                    mint,
                    programId: TOKEN_PROGRAM_ID,
                }),
                sourceOwner,
                sourceOwner,
                [],
                TOKEN_PROGRAM_ID,
            ),
        );
    }
    if (authority.equals(sourceOwner) && senderT22Balance > ZERO) {
        closeWrappedSourceInstructions.push(
            createCloseAccountInstruction(
                getAtaAddress({
                    owner: sourceOwner,
                    mint,
                    programId: TOKEN_2022_PROGRAM_ID,
                }),
                sourceOwner,
                sourceOwner,
                [],
                TOKEN_2022_PROGRAM_ID,
            ),
        );
    }

    const recipientLoadInstructions: TransactionInstruction[] = [];
    // Recipient-side load is intentionally disabled until the program allows
    // third-party load on behalf of the recipient ATA.
    // const recipientLoadInstructions = await buildLoadInstructions({
    //     rpc,
    //     payer,
    //     owner: recipient,
    //     mint,
    // });
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
            payer,
            amount: amountBigInt,
            decimals,
        });
    } else {
        const splInterfaceInfos = await getSplInterfaceInfos(rpc, mint);
        const splInterfaceInfo = splInterfaceInfos.find(
            info =>
                info.isInitialized &&
                info.tokenProgram.equals(recipientTokenProgramId),
        );
        if (!splInterfaceInfo) {
            throw new Error(
                `No initialized SPL interface found for tokenProgram ${recipientTokenProgramId.toBase58()}.`,
            );
        }
        transferInstruction = createUnwrapInstruction(
            senderAta,
            recipientAta,
            authority,
            mint,
            amountBigInt,
            splInterfaceInfo,
            decimals,
            payer,
        );
    }

    return [
        ...senderLoadInstructions,
        ...closeWrappedSourceInstructions,
        createAtaInstruction({
            payer,
            owner: recipient,
            mint,
            programId: recipientTokenProgramId,
        }),
        ...recipientLoadInstructions,
        transferInstruction,
    ];
}

/**
 * Backwards-compatible alias.
 */
export const createTransferInstructions = buildTransferInstructions;

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
        ...(await buildLoadInstructions({
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
        ...(await buildLoadInstructions({
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

export async function createFreezeInstructions(
    input: CreateFreezeInstructionsInput,
): Promise<TransactionInstruction[]> {
    return [createFreezeInstruction(input)];
}

export async function createThawInstructions(
    input: CreateThawInstructionsInput,
): Promise<TransactionInstruction[]> {
    return [createThawInstruction(input)];
}

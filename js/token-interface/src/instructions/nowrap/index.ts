import type { TransactionInstruction } from '@solana/web3.js';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import {
    createUnwrapInstruction,
    getSplInterfaceInfos,
} from '@lightprotocol/compressed-token';
import { getMintDecimals } from '../../helpers';
import { createLoadInstructionInternal } from '../../load';
import { getAtaAddress } from '../../read';
import type {
    CreateApproveInstructionsInput,
    CreateLoadInstructionsInput,
    CreateRevokeInstructionsInput,
    CreateTransferInstructionsInput,
} from '../../types';
import {
    createApproveInstruction,
    createTransferCheckedInstruction,
    createRevokeInstruction,
} from '../raw';
import {
    createAtaInstructions,
    createFreezeInstructions,
    createThawInstructions,
} from '../index';
import { getAta } from '../../account';

function toBigIntAmount(amount: number | bigint): bigint {
    return BigInt(amount.toString());
}

async function buildLoadInstructionsNoWrap(
    input: CreateLoadInstructionsInput & {
        authority?: CreateTransferInstructionsInput['authority'];
        account?: Awaited<ReturnType<typeof getAta>>;
    },
): Promise<TransactionInstruction[]> {
    const load = await createLoadInstructionInternal({
        ...input,
        wrap: false,
    });

    if (!load) {
        return [];
    }

    return load.instructions;
}

/**
 * Advanced no-wrap load helper.
 */
export async function createLoadInstructions({
    rpc,
    payer,
    owner,
    mint,
}: CreateLoadInstructionsInput): Promise<TransactionInstruction[]> {
    return buildLoadInstructionsNoWrap({
        rpc,
        payer,
        owner,
        mint,
    });
}

/**
 * No-wrap transfer flow builder.
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
    const senderLoadInstructions = await buildLoadInstructionsNoWrap({
        rpc,
        payer,
        owner: sourceOwner,
        mint,
        authority,
    });

    const recipientTokenProgramId = tokenProgram ?? LIGHT_TOKEN_PROGRAM_ID;
    const recipientAta = getAtaAddress({
        owner: recipient,
        mint,
        programId: recipientTokenProgramId,
    });
    const decimals = await getMintDecimals(rpc, mint);
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
        transferInstruction,
    ];
}

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

    return [
        ...(await buildLoadInstructionsNoWrap({
            rpc,
            payer,
            owner,
            mint,
            account,
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

    return [
        ...(await buildLoadInstructionsNoWrap({
            rpc,
            payer,
            owner,
            mint,
            account,
        })),
        createRevokeInstruction({
            tokenAccount: account.address,
            owner,
            payer,
        }),
    ];
}

export {
    createAtaInstructions,
    createFreezeInstructions,
    createThawInstructions,
};

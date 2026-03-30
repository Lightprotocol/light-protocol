import type { TransactionInstruction } from '@solana/web3.js';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import type {
    CreateApproveInstructionsInput,
    CreateBurnInstructionsInput,
    CreateFreezeInstructionsInput,
    CreateLoadInstructionsInput,
    CreateRevokeInstructionsInput,
    CreateThawInstructionsInput,
    CreateTransferInstructionsInput,
} from '../types';
import { toInstructionPlan } from '../instructions/_plan';
import { createLoadInstructions as createLoadInstructionsDefault } from '../instructions/load';
import { getAtaAddress } from '../read';
import { toBigIntAmount, getMintDecimals } from '../helpers';
import { createTransferCheckedInstruction } from '../instructions/transfer';
import { getSplInterfaces } from '../spl-interface';
import { createUnwrapInstruction } from '../instructions/unwrap';
import { createApproveInstruction } from '../instructions/approve';
import { createRevokeInstruction } from '../instructions/revoke';
import {
    createBurnInstruction,
    createBurnCheckedInstruction,
} from '../instructions/burn';
import { createFreezeInstruction } from '../instructions/freeze';
import { createThawInstruction } from '../instructions/thaw';

export * from '../instructions';

export async function createLoadInstructions(
    input: CreateLoadInstructionsInput,
): Promise<TransactionInstruction[]> {
    return createLoadInstructionsDefault({
        ...input,
        wrap: false,
    });
}

export async function createLoadInstructionPlan(
    input: CreateLoadInstructionsInput,
) {
    return toInstructionPlan(await createLoadInstructions(input));
}

export async function createTransferInstructions(
    input: CreateTransferInstructionsInput,
): Promise<TransactionInstruction[]> {
    const {
        rpc,
        payer,
        mint,
        sourceOwner,
        authority,
        recipient,
        tokenProgram,
        amount,
    } = input;
    const effectivePayer = payer ?? authority;
    const amountBigInt = toBigIntAmount(amount);
    const recipientTokenProgramId = tokenProgram ?? LIGHT_TOKEN_PROGRAM_ID;
    const [decimals, transferSplInterfaces] = await Promise.all([
        getMintDecimals(rpc, mint),
        recipientTokenProgramId.equals(LIGHT_TOKEN_PROGRAM_ID)
            ? Promise.resolve(undefined)
            : getSplInterfaces(rpc, mint),
    ]);

    const senderLoadInstructions = await createLoadInstructionsDefault({
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
            throw new Error('Missing SPL interfaces for non-light transfer path.');
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

export async function createApproveInstructions(
    input: CreateApproveInstructionsInput,
): Promise<TransactionInstruction[]> {
    const { rpc, payer, owner, mint, delegate, amount } = input;
    const tokenAccount = getAtaAddress({ owner, mint });

    return [
        ...(await createLoadInstructionsDefault({
            rpc,
            payer,
            owner,
            mint,
            wrap: false,
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

export async function createRevokeInstructions(
    input: CreateRevokeInstructionsInput,
): Promise<TransactionInstruction[]> {
    const { rpc, payer, owner, mint } = input;
    const tokenAccount = getAtaAddress({ owner, mint });

    return [
        ...(await createLoadInstructionsDefault({
            rpc,
            payer,
            owner,
            mint,
            wrap: false,
        })),
        createRevokeInstruction({
            tokenAccount,
            owner,
            payer,
        }),
    ];
}

export async function createBurnInstructions(
    input: CreateBurnInstructionsInput,
): Promise<TransactionInstruction[]> {
    const { rpc, payer, owner, mint, authority, amount, decimals } = input;
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
        ...(await createLoadInstructionsDefault({
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

export async function createFreezeInstructions(
    input: CreateFreezeInstructionsInput,
): Promise<TransactionInstruction[]> {
    const { rpc, payer, owner, mint, freezeAuthority } = input;
    const tokenAccount = getAtaAddress({ owner, mint });

    return [
        ...(await createLoadInstructionsDefault({
            rpc,
            payer,
            owner,
            mint,
            wrap: false,
        })),
        createFreezeInstruction({
            tokenAccount,
            mint,
            freezeAuthority,
        }),
    ];
}

export async function createThawInstructions(
    input: CreateThawInstructionsInput,
): Promise<TransactionInstruction[]> {
    const { rpc, payer, owner, mint, freezeAuthority } = input;
    const tokenAccount = getAtaAddress({ owner, mint });

    return [
        ...(await createLoadInstructionsDefault({
            rpc,
            payer,
            owner,
            mint,
            wrap: false,
            allowFrozen: true,
        })),
        createThawInstruction({
            tokenAccount,
            mint,
            freezeAuthority,
        }),
    ];
}

export async function createTransferInstructionPlan(
    input: CreateTransferInstructionsInput,
) {
    return toInstructionPlan(await createTransferInstructions(input));
}

export async function createApproveInstructionPlan(
    input: CreateApproveInstructionsInput,
) {
    return toInstructionPlan(await createApproveInstructions(input));
}

export async function createRevokeInstructionPlan(
    input: CreateRevokeInstructionsInput,
) {
    return toInstructionPlan(await createRevokeInstructions(input));
}

export async function createBurnInstructionPlan(
    input: CreateBurnInstructionsInput,
) {
    return toInstructionPlan(await createBurnInstructions(input));
}

export async function createFreezeInstructionPlan(
    input: CreateFreezeInstructionsInput,
) {
    return toInstructionPlan(await createFreezeInstructions(input));
}

export async function createThawInstructionPlan(
    input: CreateThawInstructionsInput,
) {
    return toInstructionPlan(await createThawInstructions(input));
}

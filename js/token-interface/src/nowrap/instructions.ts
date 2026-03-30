import type { TransactionInstruction } from '@solana/web3.js';
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
import { createTransferInstructionsNowrap } from '../instructions/transfer';
import { createApproveInstructionsNowrap } from '../instructions/approve';
import { createRevokeInstructionsNowrap } from '../instructions/revoke';
import { createBurnInstructionsNowrap } from '../instructions/burn';
import { createFreezeInstructionsNowrap } from '../instructions/freeze';
import { createThawInstructionsNowrap } from '../instructions/thaw';

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

export {
    createTransferInstructionsNowrap as createTransferInstructions,
    createApproveInstructionsNowrap as createApproveInstructions,
    createRevokeInstructionsNowrap as createRevokeInstructions,
    createBurnInstructionsNowrap as createBurnInstructions,
    createFreezeInstructionsNowrap as createFreezeInstructions,
    createThawInstructionsNowrap as createThawInstructions,
};

export async function createTransferInstructionPlan(
    input: CreateTransferInstructionsInput,
) {
    return toInstructionPlan(await createTransferInstructionsNowrap(input));
}

export async function createApproveInstructionPlan(
    input: CreateApproveInstructionsInput,
) {
    return toInstructionPlan(await createApproveInstructionsNowrap(input));
}

export async function createRevokeInstructionPlan(
    input: CreateRevokeInstructionsInput,
) {
    return toInstructionPlan(await createRevokeInstructionsNowrap(input));
}

export async function createBurnInstructionPlan(input: CreateBurnInstructionsInput) {
    return toInstructionPlan(await createBurnInstructionsNowrap(input));
}

export async function createFreezeInstructionPlan(
    input: CreateFreezeInstructionsInput,
) {
    return toInstructionPlan(await createFreezeInstructionsNowrap(input));
}

export async function createThawInstructionPlan(input: CreateThawInstructionsInput) {
    return toInstructionPlan(await createThawInstructionsNowrap(input));
}

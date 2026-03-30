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
import { _createTransferInstructions } from '../instructions/transfer';
import { _createApproveInstructions } from '../instructions/approve';
import { _createRevokeInstructions } from '../instructions/revoke';
import { _createBurnInstructions } from '../instructions/burn';
import { _createFreezeInstructions } from '../instructions/freeze';
import { _createThawInstructions } from '../instructions/thaw';

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
    return _createTransferInstructions(input, false);
}

export async function createApproveInstructions(
    input: CreateApproveInstructionsInput,
): Promise<TransactionInstruction[]> {
    return _createApproveInstructions(input, false);
}

export async function createRevokeInstructions(
    input: CreateRevokeInstructionsInput,
): Promise<TransactionInstruction[]> {
    return _createRevokeInstructions(input, false);
}

export async function createBurnInstructions(
    input: CreateBurnInstructionsInput,
): Promise<TransactionInstruction[]> {
    return _createBurnInstructions(input, false);
}

export async function createFreezeInstructions(
    input: CreateFreezeInstructionsInput,
): Promise<TransactionInstruction[]> {
    return _createFreezeInstructions(input, false);
}

export async function createThawInstructions(
    input: CreateThawInstructionsInput,
): Promise<TransactionInstruction[]> {
    return _createThawInstructions(input, false);
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

export async function createBurnInstructionPlan(input: CreateBurnInstructionsInput) {
    return toInstructionPlan(await createBurnInstructions(input));
}

export async function createFreezeInstructionPlan(
    input: CreateFreezeInstructionsInput,
) {
    return toInstructionPlan(await createFreezeInstructions(input));
}

export async function createThawInstructionPlan(input: CreateThawInstructionsInput) {
    return toInstructionPlan(await createThawInstructions(input));
}

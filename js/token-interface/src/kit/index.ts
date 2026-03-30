import type { TransactionInstruction } from '@solana/web3.js';
import {
    createTransferInstructions as createTransferInstructionsTx,
    createTransferInstructionsNowrap as createTransferInstructionsNowrapTx,
    createApproveInstructions as createApproveInstructionsTx,
    createApproveInstructionsNowrap as createApproveInstructionsNowrapTx,
    createAtaInstructions as createAtaInstructionsTx,
    createBurnInstructions as createBurnInstructionsTx,
    createBurnInstructionsNowrap as createBurnInstructionsNowrapTx,
    createFreezeInstructions as createFreezeInstructionsTx,
    createFreezeInstructionsNowrap as createFreezeInstructionsNowrapTx,
    createLoadInstructions as createLoadInstructionsTx,
    createRevokeInstructions as createRevokeInstructionsTx,
    createRevokeInstructionsNowrap as createRevokeInstructionsNowrapTx,
    createThawInstructions as createThawInstructionsTx,
    createThawInstructionsNowrap as createThawInstructionsNowrapTx,
} from '../instructions';
import type { KitInstruction } from '../instructions/_plan';
import { toKitInstructions } from '../instructions/_plan';
import type {
    CreateApproveInstructionsInput,
    CreateAtaInstructionsInput,
    CreateBurnInstructionsInput,
    CreateFreezeInstructionsInput,
    CreateLoadInstructionsInput,
    CreateRevokeInstructionsInput,
    CreateThawInstructionsInput,
    CreateTransferInstructionsInput,
} from '../types';

export type { KitInstruction };

export {
    createApproveInstructionPlan,
    createAtaInstructionPlan,
    createBurnInstructionPlan,
    createFreezeInstructionPlan,
    createLoadInstructionPlan,
    createRevokeInstructionPlan,
    createThawInstructionPlan,
    createTransferInstructionPlan,
    toInstructionPlan,
    toKitInstructions,
} from '../instructions';

function wrap(
    instructions: Promise<TransactionInstruction[]>,
): Promise<KitInstruction[]> {
    return instructions.then(ixs => toKitInstructions(ixs));
}

export async function createAtaInstructions(
    input: CreateAtaInstructionsInput,
): Promise<KitInstruction[]> {
    return toKitInstructions(createAtaInstructionsTx(input));
}

export async function createLoadInstructions(
    input: CreateLoadInstructionsInput,
): Promise<KitInstruction[]> {
    return wrap(createLoadInstructionsTx(input));
}

export async function createTransferInstructions(
    input: CreateTransferInstructionsInput,
): Promise<KitInstruction[]> {
    return wrap(createTransferInstructionsTx(input));
}

export async function createTransferInstructionsNowrap(
    input: CreateTransferInstructionsInput,
): Promise<KitInstruction[]> {
    return wrap(createTransferInstructionsNowrapTx(input));
}

export async function createApproveInstructions(
    input: CreateApproveInstructionsInput,
): Promise<KitInstruction[]> {
    return wrap(createApproveInstructionsTx(input));
}

export async function createApproveInstructionsNowrap(
    input: CreateApproveInstructionsInput,
): Promise<KitInstruction[]> {
    return wrap(createApproveInstructionsNowrapTx(input));
}

export async function createRevokeInstructions(
    input: CreateRevokeInstructionsInput,
): Promise<KitInstruction[]> {
    return wrap(createRevokeInstructionsTx(input));
}

export async function createRevokeInstructionsNowrap(
    input: CreateRevokeInstructionsInput,
): Promise<KitInstruction[]> {
    return wrap(createRevokeInstructionsNowrapTx(input));
}

export async function createFreezeInstructions(
    input: CreateFreezeInstructionsInput,
): Promise<KitInstruction[]> {
    return wrap(createFreezeInstructionsTx(input));
}

export async function createFreezeInstructionsNowrap(
    input: CreateFreezeInstructionsInput,
): Promise<KitInstruction[]> {
    return wrap(createFreezeInstructionsNowrapTx(input));
}

export async function createThawInstructions(
    input: CreateThawInstructionsInput,
): Promise<KitInstruction[]> {
    return wrap(createThawInstructionsTx(input));
}

export async function createThawInstructionsNowrap(
    input: CreateThawInstructionsInput,
): Promise<KitInstruction[]> {
    return wrap(createThawInstructionsNowrapTx(input));
}

export async function createBurnInstructions(
    input: CreateBurnInstructionsInput,
): Promise<KitInstruction[]> {
    return wrap(createBurnInstructionsTx(input));
}

export async function createBurnInstructionsNowrap(
    input: CreateBurnInstructionsInput,
): Promise<KitInstruction[]> {
    return wrap(createBurnInstructionsNowrapTx(input));
}

export type {
    CreateApproveInstructionsInput,
    CreateAtaInstructionsInput,
    CreateBurnInstructionsInput,
    CreateFreezeInstructionsInput,
    CreateLoadInstructionsInput,
    CreateRevokeInstructionsInput,
    CreateThawInstructionsInput,
    CreateTransferInstructionsInput,
};

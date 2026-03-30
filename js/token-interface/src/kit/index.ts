import type { TransactionInstruction } from '@solana/web3.js';
import {
    createTransferInstructions as createTransferInstructionsTx,
    createApproveInstructions as createApproveInstructionsTx,
    createAtaInstructions as createAtaInstructionsTx,
    createBurnInstructions as createBurnInstructionsTx,
    createFreezeInstructions as createFreezeInstructionsTx,
    createLoadInstructions as createLoadInstructionsTx,
    createRevokeInstructions as createRevokeInstructionsTx,
    createThawInstructions as createThawInstructionsTx,
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

export async function createApproveInstructions(
    input: CreateApproveInstructionsInput,
): Promise<KitInstruction[]> {
    return wrap(createApproveInstructionsTx(input));
}

export async function createRevokeInstructions(
    input: CreateRevokeInstructionsInput,
): Promise<KitInstruction[]> {
    return wrap(createRevokeInstructionsTx(input));
}

export async function createFreezeInstructions(
    input: CreateFreezeInstructionsInput,
): Promise<KitInstruction[]> {
    return wrap(createFreezeInstructionsTx(input));
}

export async function createThawInstructions(
    input: CreateThawInstructionsInput,
): Promise<KitInstruction[]> {
    return wrap(createThawInstructionsTx(input));
}

export async function createBurnInstructions(
    input: CreateBurnInstructionsInput,
): Promise<KitInstruction[]> {
    return wrap(createBurnInstructionsTx(input));
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

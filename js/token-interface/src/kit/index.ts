import { fromLegacyTransactionInstruction } from '@solana/compat';
import {
    sequentialInstructionPlan,
    type InstructionPlan,
} from '@solana/kit';
import type { TransactionInstruction } from '@solana/web3.js';
import {
    createApproveInstructions as createLegacyApproveInstructions,
    buildTransferInstructions as buildLegacyTransferInstructions,
    createAtaInstructions as createLegacyAtaInstructions,
    createFreezeInstructions as createLegacyFreezeInstructions,
    createLoadInstructions as createLegacyLoadInstructions,
    createRevokeInstructions as createLegacyRevokeInstructions,
    createThawInstructions as createLegacyThawInstructions,
} from '../instructions';
import type {
    CreateApproveInstructionsInput,
    CreateAtaInstructionsInput,
    CreateFreezeInstructionsInput,
    CreateLoadInstructionsInput,
    CreateRevokeInstructionsInput,
    CreateThawInstructionsInput,
    CreateTransferInstructionsInput,
} from '../types';

export type KitInstruction = ReturnType<typeof fromLegacyTransactionInstruction>;

export function toKitInstructions(
    instructions: TransactionInstruction[],
): KitInstruction[] {
    return instructions.map(instruction =>
        fromLegacyTransactionInstruction(instruction),
    );
}

export async function createAtaInstructions(
    input: CreateAtaInstructionsInput,
): Promise<KitInstruction[]> {
    return toKitInstructions(await createLegacyAtaInstructions(input));
}

/**
 * Advanced: standalone load (decompress) instructions for an ATA, plus create-ATA if missing.
 * Prefer the canonical builders (`buildTransferInstructions`, `createApproveInstructions`,
 * `createRevokeInstructions`, …), which prepend load automatically when needed.
 */
export async function createLoadInstructions(
    input: CreateLoadInstructionsInput,
): Promise<KitInstruction[]> {
    return toKitInstructions(await createLegacyLoadInstructions(input));
}

/**
 * Canonical Kit instruction-array builder.
 * Returns Kit instructions (not an InstructionPlan).
 */
export async function buildTransferInstructions(
    input: CreateTransferInstructionsInput,
): Promise<KitInstruction[]> {
    return toKitInstructions(await buildLegacyTransferInstructions(input));
}

/**
 * Backwards-compatible alias.
 */
export const createTransferInstructions = buildTransferInstructions;

/**
 * Canonical Kit plan builder.
 */
export async function getTransferInstructionPlan(
    input: CreateTransferInstructionsInput,
): Promise<InstructionPlan> {
    return sequentialInstructionPlan(await buildTransferInstructions(input));
}

export async function createApproveInstructions(
    input: CreateApproveInstructionsInput,
): Promise<KitInstruction[]> {
    return toKitInstructions(await createLegacyApproveInstructions(input));
}

export async function createRevokeInstructions(
    input: CreateRevokeInstructionsInput,
): Promise<KitInstruction[]> {
    return toKitInstructions(await createLegacyRevokeInstructions(input));
}

export async function createFreezeInstructions(
    input: CreateFreezeInstructionsInput,
): Promise<KitInstruction[]> {
    return toKitInstructions(await createLegacyFreezeInstructions(input));
}

export async function createThawInstructions(
    input: CreateThawInstructionsInput,
): Promise<KitInstruction[]> {
    return toKitInstructions(await createLegacyThawInstructions(input));
}

export type {
    CreateApproveInstructionsInput,
    CreateAtaInstructionsInput,
    CreateFreezeInstructionsInput,
    CreateLoadInstructionsInput,
    CreateRevokeInstructionsInput,
    CreateThawInstructionsInput,
    CreateTransferInstructionsInput,
};

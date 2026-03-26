import { fromLegacyTransactionInstruction } from '@solana/compat';
import {
    sequentialInstructionPlan,
    type InstructionPlan,
} from '@solana/instruction-plans';
import type { TransactionInstruction } from '@solana/web3.js';

export type KitInstruction = ReturnType<typeof fromLegacyTransactionInstruction>;

export function toKitInstructions(
    instructions: TransactionInstruction[],
): KitInstruction[] {
    return instructions.map(instruction =>
        fromLegacyTransactionInstruction(instruction),
    );
}

export function toInstructionPlan(
    instructions: TransactionInstruction[],
): InstructionPlan {
    return sequentialInstructionPlan(toKitInstructions(instructions));
}

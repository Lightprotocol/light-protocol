/**
 * High-level transaction builders that wire load → select → proof → instruction.
 *
 * These bridge the gap between token-client (data loading) and token-sdk (instruction building).
 */

import type { Address } from '@solana/addresses';
import type { Instruction } from '@solana/instructions';

import type { LightIndexer } from './indexer.js';
import {
    loadTokenAccountsForTransfer,
    type InputTokenAccount,
    type LoadTokenAccountsOptions,
} from './load.js';

import type { ValidityProofWithContext } from '@lightprotocol/token-sdk';
import { createTransferInstruction } from '@lightprotocol/token-sdk';

/**
 * Result of building a transfer instruction with loaded account data.
 */
export interface BuildTransferResult {
    /** The transfer instruction(s) to include in the transaction */
    instructions: Instruction[];
    /** The input token accounts used */
    inputs: InputTokenAccount[];
    /** The validity proof for the inputs */
    proof: ValidityProofWithContext;
}

/**
 * Builds a transfer instruction by loading accounts, selecting inputs,
 * fetching a validity proof, and creating the instruction.
 *
 * This is the primary high-level API that wires together the full flow:
 * 1. Fetch token accounts from the indexer
 * 2. Select accounts that cover the requested amount
 * 3. Fetch a validity proof for the selected accounts
 * 4. Create the transfer instruction
 *
 * @param indexer - Light indexer client
 * @param params - Transfer parameters
 * @returns The instruction, inputs, and proof
 *
 * @example
 * ```typescript
 * const result = await buildTransferInstruction(indexer, {
 *     owner: ownerAddress,
 *     mint: mintAddress,
 *     destination: destinationAta,
 *     amount: 1000n,
 *     authority: ownerAddress,
 * });
 * // result.instructions contains the transfer instruction
 * // result.proof contains the validity proof for the transaction
 * ```
 */
export async function buildTransferInstruction(
    indexer: LightIndexer,
    params: {
        owner: Address;
        mint: Address;
        destination: Address;
        amount: bigint;
        authority: Address;
        maxTopUp?: number;
        feePayer?: Address;
    },
): Promise<BuildTransferResult> {
    const options: LoadTokenAccountsOptions = { mint: params.mint };

    // Load and select accounts, fetch proof
    const loaded = await loadTokenAccountsForTransfer(
        indexer,
        params.owner,
        params.amount,
        options,
    );

    // Build the transfer instruction using the first input's ATA as source
    // For multi-input transfers, the SDK consumer would use Transfer2 instead.
    const instruction = createTransferInstruction({
        source: loaded.inputs[0].merkleContext.tree,
        destination: params.destination,
        amount: params.amount,
        authority: params.authority,
        maxTopUp: params.maxTopUp,
        feePayer: params.feePayer,
    });

    return {
        instructions: [instruction],
        inputs: loaded.inputs,
        proof: loaded.proof,
    };
}

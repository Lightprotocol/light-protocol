/**
 * Transfer interface - auto-routing between light-to-light, light-to-SPL, and SPL-to-light.
 */

import type { Address } from '@solana/addresses';
import type { Instruction } from '@solana/instructions';

import { determineTransferType } from '../utils/validation.js';
import { createTransferInstruction } from './transfer.js';

/**
 * Transfer type for routing.
 */
export type TransferType =
    | 'light-to-light'
    | 'light-to-spl'
    | 'spl-to-light'
    | 'spl-to-spl';

/**
 * Parameters for transfer interface.
 */
export interface TransferInterfaceParams {
    /** Source account owner (to determine if Light or SPL) */
    sourceOwner: Address;
    /** Destination account owner (to determine if Light or SPL) */
    destOwner: Address;
    /** Source token account */
    source: Address;
    /** Destination token account */
    destination: Address;
    /** Amount to transfer */
    amount: bigint;
    /** Authority for the transfer */
    authority: Address;
    /** Mint address (for routing and pools) */
    mint: Address;
    /** Maximum top-up for rent (optional) */
    maxTopUp?: number;
}

/**
 * Result of transfer interface routing.
 */
export interface TransferInterfaceResult {
    /** The determined transfer type */
    transferType: TransferType;
    /** The instruction(s) to execute */
    instructions: Instruction[];
}

/**
 * Creates transfer instruction(s) with automatic routing.
 *
 * Routes transfers based on account ownership:
 * - Light-to-Light: Direct CToken transfer
 * - Light-to-SPL: Decompress to SPL (requires Transfer2)
 * - SPL-to-Light: Compress from SPL (requires Transfer2)
 * - SPL-to-SPL: Falls through to SPL Token program
 *
 * @param params - Transfer interface parameters
 * @returns The transfer type and instruction(s)
 */
export function createTransferInterfaceInstruction(
    params: TransferInterfaceParams,
): TransferInterfaceResult {
    const transferType = determineTransferType(
        params.sourceOwner,
        params.destOwner,
    );

    switch (transferType) {
        case 'light-to-light':
            return {
                transferType,
                instructions: [
                    createTransferInstruction({
                        source: params.source,
                        destination: params.destination,
                        amount: params.amount,
                        authority: params.authority,
                        maxTopUp: params.maxTopUp,
                    }),
                ],
            };

        case 'light-to-spl':
            throw new Error(
                'Light-to-SPL transfer requires Transfer2 with DECOMPRESS mode. ' +
                    'Use createTransfer2Instruction() with createDecompress() or ' +
                    'createDecompressSpl() to build the Compression struct.',
            );

        case 'spl-to-light':
            throw new Error(
                'SPL-to-Light transfer requires Transfer2 with COMPRESS mode. ' +
                    'Use createTransfer2Instruction() with createCompress() or ' +
                    'createCompressSpl() to build the Compression struct.',
            );

        case 'spl-to-spl':
            throw new Error(
                'SPL-to-SPL transfers should use the SPL Token program directly.',
            );
    }
}

/**
 * Helper to determine if a transfer requires compression operations.
 *
 * @param sourceOwner - Source account owner
 * @param destOwner - Destination account owner
 * @returns True if the transfer crosses the Light/SPL boundary
 */
export function requiresCompression(
    sourceOwner: Address,
    destOwner: Address,
): boolean {
    const transferType = determineTransferType(sourceOwner, destOwner);
    return transferType === 'light-to-spl' || transferType === 'spl-to-light';
}

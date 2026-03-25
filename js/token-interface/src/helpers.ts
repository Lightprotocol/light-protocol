import {
    type InterfaceOptions,
    getMintInterface,
} from '@lightprotocol/compressed-token';
import { ComputeBudgetProgram, PublicKey } from '@solana/web3.js';
import type { Rpc } from '@lightprotocol/stateless.js';
import type { TransactionInstruction } from '@solana/web3.js';
import { MultiTransactionNotSupportedError } from './errors';

export async function getMintDecimals(
    rpc: Rpc,
    mint: PublicKey,
): Promise<number> {
    const mintInterface = await getMintInterface(rpc, mint);
    return mintInterface.mint.decimals;
}

export function toInterfaceOptions(
    owner: PublicKey,
    authority?: PublicKey,
    wrap = false,
): InterfaceOptions | undefined {
    if ((!authority || authority.equals(owner)) && !wrap) {
        return undefined;
    }

    const options: InterfaceOptions = {};
    if (wrap) {
        options.wrap = true;
    }
    if (authority && !authority.equals(owner)) {
        options.delegatePubkey = authority;
    }

    return options;
}

export function normalizeInstructionBatches(
    operation: string,
    batches: TransactionInstruction[][],
): TransactionInstruction[] {
    if (batches.length === 0) {
        return [];
    }

    if (batches.length > 1) {
        throw new MultiTransactionNotSupportedError(operation, batches.length);
    }

    return batches[0].filter(
        instruction =>
            !instruction.programId.equals(ComputeBudgetProgram.programId),
    );
}

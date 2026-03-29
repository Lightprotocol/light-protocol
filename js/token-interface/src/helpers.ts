import type { LoadOptions } from './load-options';
import { getMint } from './read';
import { ComputeBudgetProgram, PublicKey } from '@solana/web3.js';
import type { Rpc } from '@lightprotocol/stateless.js';
import type { TransactionInstruction } from '@solana/web3.js';
import { MultiTransactionNotSupportedError } from './errors';

export async function getMintDecimals(
    rpc: Rpc,
    mint: PublicKey,
): Promise<number> {
    const mintInfo = await getMint(rpc, mint);
    return mintInfo.mint.decimals;
}

export function toLoadOptions(
    owner: PublicKey,
    authority?: PublicKey,
    wrap = false,
): LoadOptions | undefined {
    if ((!authority || authority.equals(owner)) && !wrap) {
        return undefined;
    }

    const options: LoadOptions = {};
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

export function toBigIntAmount(amount: number | bigint): bigint {
    return BigInt(amount.toString());
}

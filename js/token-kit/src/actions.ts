/**
 * High-level transaction builders that wire load → select → proof → instruction.
 *
 * These bridge the gap between token-client (data loading) and token-sdk (instruction building).
 */

import type { Address } from '@solana/addresses';
import { AccountRole, type Instruction, type AccountMeta } from '@solana/instructions';

import type { LightIndexer } from './indexer.js';
import {
    loadTokenAccountsForTransfer,
    getOutputTreeInfo,
    type InputTokenAccount,
    type LoadTokenAccountsOptions,
} from './load.js';

import {
    IndexerError,
    IndexerErrorCode,
    type ValidityProofWithContext,
} from './client/index.js';
import { createTransfer2Instruction } from './instructions/index.js';

/**
 * Result of building a compressed transfer instruction with loaded account data.
 */
export interface BuildTransferResult {
    /** The transfer instruction to include in the transaction */
    instruction: Instruction;
    /** The input token accounts used */
    inputs: InputTokenAccount[];
    /** The validity proof for the inputs */
    proof: ValidityProofWithContext;
    /** Total amount available (may exceed requested amount; change goes back to sender) */
    totalInputAmount: bigint;
}

/**
 * Builds a compressed token transfer (Transfer2) instruction by loading accounts,
 * selecting inputs, fetching a validity proof, and creating the instruction.
 *
 * This is the primary high-level API for compressed token transfers.
 *
 * Flow:
 * 1. Fetch token accounts from the indexer
 * 2. Select accounts that cover the requested amount
 * 3. Fetch a validity proof for the selected accounts
 * 4. Create the Transfer2 instruction with proof and merkle contexts
 *
 * @param indexer - Light indexer client
 * @param params - Transfer parameters
 * @returns The instruction, inputs, and proof
 *
 * @example
 * ```typescript
 * const result = await buildCompressedTransfer(indexer, {
 *     owner: ownerAddress,
 *     mint: mintAddress,
 *     amount: 1000n,
 *     recipientOwner: recipientAddress,
 *     feePayer: payerAddress,
 * });
 * // result.instruction is the Transfer2 instruction
 * ```
 */
export async function buildCompressedTransfer(
    indexer: LightIndexer,
    params: {
        /** Token account owner (sender) */
        owner: Address;
        /** Token mint */
        mint: Address;
        /** Amount to transfer */
        amount: bigint;
        /** Recipient owner address */
        recipientOwner: Address;
        /** Fee payer address (signer, writable) */
        feePayer: Address;
        /** Maximum top-up amount for rent (optional) */
        maxTopUp?: number;
        /** Maximum number of input accounts (default: 4) */
        maxInputs?: number;
    },
): Promise<BuildTransferResult> {
    const options: LoadTokenAccountsOptions = {
        mint: params.mint,
        maxInputs: params.maxInputs,
    };

    // Load and select accounts, fetch proof
    const loaded = await loadTokenAccountsForTransfer(
        indexer,
        params.owner,
        params.amount,
        options,
    );
    if (loaded.inputs.length === 0) {
        throw new IndexerError(
            IndexerErrorCode.InvalidResponse,
            'No inputs were selected for transfer',
        );
    }

    const hashToKey = (hash: Uint8Array): string =>
        Array.from(hash, (b) => b.toString(16).padStart(2, '0')).join('');

    const proofRootIndexByHash = new Map<string, number>();
    for (const proofInput of loaded.proof.accounts) {
        const key = hashToKey(proofInput.hash);
        if (proofRootIndexByHash.has(key)) {
            throw new IndexerError(
                IndexerErrorCode.InvalidResponse,
                `Duplicate proof entry for input hash ${key}`,
            );
        }
        const rootIndex = proofInput.rootIndex.rootIndex;
        if (!Number.isInteger(rootIndex) || rootIndex < 0 || rootIndex > 65535) {
            throw new IndexerError(
                IndexerErrorCode.InvalidResponse,
                `Invalid rootIndex ${rootIndex} for input hash ${key}`,
            );
        }
        proofRootIndexByHash.set(key, rootIndex);
    }

    // Build packed accounts matching V3 ordering: trees/queues first (owned by
    // account_compression program), then token-related accounts (mint, owner, etc.).
    const packedAddressMap = new Map<string, number>();
    const packedAccounts: AccountMeta[] = [];

    function getOrAddPacked(addr: Address, role: AccountRole): number {
        const existing = packedAddressMap.get(addr as string);
        if (existing !== undefined) return existing;
        const idx = packedAccounts.length;
        packedAddressMap.set(addr as string, idx);
        packedAccounts.push({ address: addr, role });
        return idx;
    }

    // 1. Add merkle tree/queue pairs first (writable, account_compression-owned)
    for (const input of loaded.inputs) {
        getOrAddPacked(input.merkleContext.tree, AccountRole.WRITABLE);
        getOrAddPacked(input.merkleContext.queue, AccountRole.WRITABLE);
    }

    // 2. Add output queue (writable) — rollover-aware
    const outputTreeInfo = getOutputTreeInfo(
        loaded.inputs[0].tokenAccount.account.treeInfo,
    );
    const outputQueueIdx = getOrAddPacked(
        outputTreeInfo.queue,
        AccountRole.WRITABLE,
    );

    // 3. Add mint (readonly)
    const mintIdx = getOrAddPacked(params.mint, AccountRole.READONLY);

    // 4. Add owner (readonly)
    const ownerIdx = getOrAddPacked(params.owner, AccountRole.READONLY);

    // 5. Add recipient (readonly)
    const recipientIdx = getOrAddPacked(
        params.recipientOwner,
        AccountRole.READONLY,
    );

    // Build input token data from loaded accounts
    const inTokenData = loaded.inputs.map((input) => {
        const treeIdx = getOrAddPacked(
            input.merkleContext.tree,
            AccountRole.WRITABLE,
        );
        const queueIdx = getOrAddPacked(
            input.merkleContext.queue,
            AccountRole.WRITABLE,
        );

        const inputHashKey = hashToKey(input.tokenAccount.account.hash);
        const rootIndex = proofRootIndexByHash.get(inputHashKey);
        if (rootIndex === undefined) {
            throw new IndexerError(
                IndexerErrorCode.InvalidResponse,
                `Missing proof account for selected input hash ${inputHashKey}`,
            );
        }

        const delegateAddress = input.tokenAccount.token.delegate;
        const hasDelegate = delegateAddress !== null;
        const delegateIdx = hasDelegate
            ? getOrAddPacked(delegateAddress, AccountRole.READONLY)
            : 0;

        return {
            owner: ownerIdx,
            amount: input.tokenAccount.token.amount,
            hasDelegate,
            delegate: delegateIdx,
            mint: mintIdx,
            version: 3, // V2 token accounts
            merkleContext: {
                merkleTreePubkeyIndex: treeIdx,
                queuePubkeyIndex: queueIdx,
                leafIndex: input.merkleContext.leafIndex,
                proveByIndex: input.merkleContext.proveByIndex,
            },
            rootIndex,
        };
    });

    // Build output token data
    // Output 0: recipient gets the requested amount
    // Output 1: change back to sender (if any)
    const outTokenData = [
        {
            owner: recipientIdx,
            amount: params.amount,
            hasDelegate: false,
            delegate: 0,
            mint: mintIdx,
            version: 3,
        },
    ];

    if (loaded.totalAmount > params.amount) {
        outTokenData.push({
            owner: ownerIdx,
            amount: loaded.totalAmount - params.amount,
            hasDelegate: false,
            delegate: 0,
            mint: mintIdx,
            version: 3,
        });
    }

    const instruction = createTransfer2Instruction({
        feePayer: params.feePayer,
        packedAccounts,
        data: {
            withTransactionHash: false,
            withLamportsChangeAccountMerkleTreeIndex: false,
            lamportsChangeAccountMerkleTreeIndex: 0,
            lamportsChangeAccountOwnerIndex: ownerIdx,
            outputQueue: outputQueueIdx,
            maxTopUp: params.maxTopUp ?? 65535,
            cpiContext: null,
            compressions: null,
            proof: loaded.proof.proof,
            inTokenData,
            outTokenData,
            inLamports: null,
            outLamports: null,
            inTlv: null,
            outTlv: null,
        },
    });

    return {
        instruction,
        inputs: loaded.inputs,
        proof: loaded.proof,
        totalInputAmount: loaded.totalAmount,
    };
}

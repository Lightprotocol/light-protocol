import {
    PublicKey,
    ComputeBudgetProgram,
    TransactionInstruction,
} from '@solana/web3.js';
import {
    Rpc,
    assertBetaEnabled,
    LIGHT_TOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import {
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    createApproveInstruction as createSplApproveInstruction,
    createRevokeInstruction as createSplRevokeInstruction,
} from '@solana/spl-token';
import BN from 'bn.js';
import {
    createLightTokenApproveInstruction,
    createLightTokenRevokeInstruction,
} from './approve-revoke';
import {
    getAtaInterface as _getAtaInterface,
    checkNotFrozen,
} from '../get-account-interface';
import {
    _buildLoadBatches,
    calculateLoadBatchComputeUnits,
    type InternalLoadBatch,
} from './load-ata';
import { calculateCombinedCU } from './calculate-combined-cu';
import { assertTransactionSizeWithinLimit } from '../utils/estimate-tx-size';

const APPROVE_BASE_CU = 10_000;

function calculateApproveCU(loadBatch: InternalLoadBatch | null): number {
    return calculateCombinedCU(APPROVE_BASE_CU, loadBatch);
}

/**
 * Build instruction batches for approving a delegate on an ATA.
 *
 * Supports light-token, SPL, and Token-2022 mints.
 * Returns `TransactionInstruction[][]`. Send [0..n-2] in parallel, then [n-1].
 *
 * @param rpc          RPC connection
 * @param payer        Fee payer public key
 * @param mint         Mint address
 * @param tokenAccount ATA address
 * @param delegate     Delegate to approve
 * @param amount       Amount to delegate
 * @param owner        Owner public key
 * @param decimals     Token decimals
 * @param programId    Token program ID (default: LIGHT_TOKEN_PROGRAM_ID)
 * @param wrap         When true and mint is SPL/T22, wrap into light-token then approve
 * @returns Instruction batches
 */
export async function createApproveInterfaceInstructions(
    rpc: Rpc,
    payer: PublicKey,
    mint: PublicKey,
    tokenAccount: PublicKey,
    delegate: PublicKey,
    amount: number | bigint | BN,
    owner: PublicKey,
    decimals: number,
    programId: PublicKey = LIGHT_TOKEN_PROGRAM_ID,
    wrap = false,
): Promise<TransactionInstruction[][]> {
    assertBetaEnabled();

    const amountBigInt = BigInt(amount.toString());

    const isSplOrT22 =
        programId.equals(TOKEN_PROGRAM_ID) ||
        programId.equals(TOKEN_2022_PROGRAM_ID);

    const accountInterface = await _getAtaInterface(
        rpc,
        tokenAccount,
        owner,
        mint,
        undefined,
        programId.equals(LIGHT_TOKEN_PROGRAM_ID) ? undefined : programId,
        wrap,
    );

    checkNotFrozen(accountInterface, 'approve');

    if (isSplOrT22 && !wrap) {
        const approveIx = createSplApproveInstruction(
            tokenAccount,
            delegate,
            owner,
            amountBigInt,
            [],
            programId,
        );

        const numSigners = payer.equals(owner) ? 1 : 2;
        const txIxs = [
            ComputeBudgetProgram.setComputeUnitLimit({
                units: APPROVE_BASE_CU,
            }),
            approveIx,
        ];
        assertTransactionSizeWithinLimit(txIxs, numSigners, 'Batch');
        return [txIxs];
    }

    // Light-token path: load cold accounts if needed
    const internalBatches = await _buildLoadBatches(
        rpc,
        payer,
        accountInterface,
        undefined,
        wrap,
        tokenAccount,
        undefined,
        owner,
        decimals,
    );

    const approveIx = createLightTokenApproveInstruction(
        tokenAccount,
        delegate,
        owner,
        amountBigInt,
        payer,
    );

    const numSigners = payer.equals(owner) ? 1 : 2;

    if (internalBatches.length === 0) {
        const cu = calculateApproveCU(null);
        const txIxs = [
            ComputeBudgetProgram.setComputeUnitLimit({ units: cu }),
            approveIx,
        ];
        assertTransactionSizeWithinLimit(txIxs, numSigners, 'Batch');
        return [txIxs];
    }

    if (internalBatches.length === 1) {
        const batch = internalBatches[0];
        const cu = calculateApproveCU(batch);
        const txIxs = [
            ComputeBudgetProgram.setComputeUnitLimit({ units: cu }),
            ...batch.instructions,
            approveIx,
        ];
        assertTransactionSizeWithinLimit(txIxs, numSigners, 'Batch');
        return [txIxs];
    }

    const result: TransactionInstruction[][] = [];

    for (let i = 0; i < internalBatches.length - 1; i++) {
        const batch = internalBatches[i];
        const cu = calculateLoadBatchComputeUnits(batch);
        const txIxs = [
            ComputeBudgetProgram.setComputeUnitLimit({ units: cu }),
            ...batch.instructions,
        ];
        assertTransactionSizeWithinLimit(txIxs, numSigners, 'Batch');
        result.push(txIxs);
    }

    const lastBatch = internalBatches[internalBatches.length - 1];
    const lastCu = calculateApproveCU(lastBatch);
    const lastTxIxs = [
        ComputeBudgetProgram.setComputeUnitLimit({ units: lastCu }),
        ...lastBatch.instructions,
        approveIx,
    ];
    assertTransactionSizeWithinLimit(lastTxIxs, numSigners, 'Batch');
    result.push(lastTxIxs);

    return result;
}

/**
 * Build instruction batches for revoking delegation on an ATA.
 *
 * Supports light-token, SPL, and Token-2022 mints.
 * Returns `TransactionInstruction[][]`. Send [0..n-2] in parallel, then [n-1].
 *
 * @param rpc          RPC connection
 * @param payer        Fee payer public key
 * @param mint         Mint address
 * @param tokenAccount ATA address
 * @param owner        Owner public key
 * @param decimals     Token decimals
 * @param programId    Token program ID (default: LIGHT_TOKEN_PROGRAM_ID)
 * @param wrap         When true and mint is SPL/T22, wrap into light-token then revoke
 * @returns Instruction batches
 */
export async function createRevokeInterfaceInstructions(
    rpc: Rpc,
    payer: PublicKey,
    mint: PublicKey,
    tokenAccount: PublicKey,
    owner: PublicKey,
    decimals: number,
    programId: PublicKey = LIGHT_TOKEN_PROGRAM_ID,
    wrap = false,
): Promise<TransactionInstruction[][]> {
    assertBetaEnabled();

    const isSplOrT22 =
        programId.equals(TOKEN_PROGRAM_ID) ||
        programId.equals(TOKEN_2022_PROGRAM_ID);

    const accountInterface = await _getAtaInterface(
        rpc,
        tokenAccount,
        owner,
        mint,
        undefined,
        programId.equals(LIGHT_TOKEN_PROGRAM_ID) ? undefined : programId,
        wrap,
    );

    checkNotFrozen(accountInterface, 'revoke');

    if (isSplOrT22 && !wrap) {
        const revokeIx = createSplRevokeInstruction(
            tokenAccount,
            owner,
            [],
            programId,
        );

        const numSigners = payer.equals(owner) ? 1 : 2;
        const txIxs = [
            ComputeBudgetProgram.setComputeUnitLimit({
                units: APPROVE_BASE_CU,
            }),
            revokeIx,
        ];
        assertTransactionSizeWithinLimit(txIxs, numSigners, 'Batch');
        return [txIxs];
    }

    // Light-token path: load cold accounts if needed
    const internalBatches = await _buildLoadBatches(
        rpc,
        payer,
        accountInterface,
        undefined,
        wrap,
        tokenAccount,
        undefined,
        owner,
        decimals,
    );

    const revokeIx = createLightTokenRevokeInstruction(
        tokenAccount,
        owner,
        payer,
    );

    const numSigners = payer.equals(owner) ? 1 : 2;

    if (internalBatches.length === 0) {
        const cu = calculateApproveCU(null);
        const txIxs = [
            ComputeBudgetProgram.setComputeUnitLimit({ units: cu }),
            revokeIx,
        ];
        assertTransactionSizeWithinLimit(txIxs, numSigners, 'Batch');
        return [txIxs];
    }

    if (internalBatches.length === 1) {
        const batch = internalBatches[0];
        const cu = calculateApproveCU(batch);
        const txIxs = [
            ComputeBudgetProgram.setComputeUnitLimit({ units: cu }),
            ...batch.instructions,
            revokeIx,
        ];
        assertTransactionSizeWithinLimit(txIxs, numSigners, 'Batch');
        return [txIxs];
    }

    const result: TransactionInstruction[][] = [];

    for (let i = 0; i < internalBatches.length - 1; i++) {
        const batch = internalBatches[i];
        const cu = calculateLoadBatchComputeUnits(batch);
        const txIxs = [
            ComputeBudgetProgram.setComputeUnitLimit({ units: cu }),
            ...batch.instructions,
        ];
        assertTransactionSizeWithinLimit(txIxs, numSigners, 'Batch');
        result.push(txIxs);
    }

    const lastBatch = internalBatches[internalBatches.length - 1];
    const lastCu = calculateApproveCU(lastBatch);
    const lastTxIxs = [
        ComputeBudgetProgram.setComputeUnitLimit({ units: lastCu }),
        ...lastBatch.instructions,
        revokeIx,
    ];
    assertTransactionSizeWithinLimit(lastTxIxs, numSigners, 'Batch');
    result.push(lastTxIxs);

    return result;
}

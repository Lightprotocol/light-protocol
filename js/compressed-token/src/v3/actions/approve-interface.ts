import {
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
    ComputeBudgetProgram,
    TransactionInstruction,
} from '@solana/web3.js';
import {
    Rpc,
    buildAndSignTx,
    sendAndConfirmTx,
    dedupeSigner,
    assertBetaEnabled,
} from '@lightprotocol/stateless.js';
import BN from 'bn.js';
import {
    createLightTokenApproveInstruction,
    createLightTokenRevokeInstruction,
} from '../instructions/approve-revoke';
import { getAssociatedTokenAddressInterface } from '../get-associated-token-address-interface';
import { getMintInterface } from '../get-mint-interface';
import {
    getAtaInterface as _getAtaInterface,
    checkNotFrozen,
} from '../get-account-interface';
import {
    _buildLoadBatches,
    calculateLoadBatchComputeUnits,
    type InternalLoadBatch,
} from '../instructions/load-ata';
import { calculateCombinedCU } from '../instructions/calculate-combined-cu';
import { assertTransactionSizeWithinLimit } from '../utils/estimate-tx-size';
import { sliceLast } from './slice-last';

const APPROVE_BASE_CU = 10_000;

function calculateApproveCU(loadBatch: InternalLoadBatch | null): number {
    return calculateCombinedCU(APPROVE_BASE_CU, loadBatch);
}

/**
 * Approve a delegate for a light-token associated token account.
 *
 * Loads cold accounts if needed, then sends the approve instruction.
 *
 * @param rpc            RPC connection
 * @param payer          Fee payer (signer)
 * @param tokenAccount   Light-token ATA address
 * @param mint           Mint address
 * @param delegate       Delegate to approve
 * @param amount         Amount to delegate
 * @param owner          Owner of the token account (signer)
 * @param confirmOptions Optional confirm options
 * @returns Transaction signature
 */
export async function approveInterface(
    rpc: Rpc,
    payer: Signer,
    tokenAccount: PublicKey,
    mint: PublicKey,
    delegate: PublicKey,
    amount: number | bigint | BN,
    owner: Signer,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    const expectedAta = getAssociatedTokenAddressInterface(
        mint,
        owner.publicKey,
    );
    if (!tokenAccount.equals(expectedAta)) {
        throw new Error(
            `Token account mismatch. Expected ${expectedAta.toBase58()}, got ${tokenAccount.toBase58()}`,
        );
    }

    const mintInterface = await getMintInterface(rpc, mint);
    const batches = await createApproveInterfaceInstructions(
        rpc,
        payer.publicKey,
        mint,
        tokenAccount,
        delegate,
        amount,
        owner.publicKey,
        mintInterface.mint.decimals,
    );

    const additionalSigners = dedupeSigner(payer, [owner]);
    const { rest: loads, last: approveIxs } = sliceLast(batches);

    await Promise.all(
        loads.map(async ixs => {
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(
                ixs,
                payer,
                blockhash,
                additionalSigners,
            );
            return sendAndConfirmTx(rpc, tx, confirmOptions);
        }),
    );

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(
        approveIxs,
        payer,
        blockhash,
        additionalSigners,
    );
    return sendAndConfirmTx(rpc, tx, confirmOptions);
}

/**
 * Build instruction batches for approving a delegate on a light-token ATA.
 *
 * Returns `TransactionInstruction[][]`. Send [0..n-2] in parallel, then [n-1].
 *
 * @param rpc          RPC connection
 * @param payer        Fee payer public key
 * @param mint         Mint address
 * @param tokenAccount Light-token ATA address
 * @param delegate     Delegate to approve
 * @param amount       Amount to delegate
 * @param owner        Owner public key
 * @param decimals     Token decimals
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
): Promise<TransactionInstruction[][]> {
    assertBetaEnabled();

    const amountBigInt = BigInt(amount.toString());

    const accountInterface = await _getAtaInterface(
        rpc,
        tokenAccount,
        owner,
        mint,
    );

    checkNotFrozen(accountInterface, 'approve');

    const internalBatches = await _buildLoadBatches(
        rpc,
        payer,
        accountInterface,
        undefined,
        false,
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
 * Revoke delegation for a light-token associated token account.
 *
 * Loads cold accounts if needed, then sends the revoke instruction.
 *
 * @param rpc            RPC connection
 * @param payer          Fee payer (signer)
 * @param tokenAccount   Light-token ATA address
 * @param mint           Mint address
 * @param owner          Owner of the token account (signer)
 * @param confirmOptions Optional confirm options
 * @returns Transaction signature
 */
export async function revokeInterface(
    rpc: Rpc,
    payer: Signer,
    tokenAccount: PublicKey,
    mint: PublicKey,
    owner: Signer,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    const expectedAta = getAssociatedTokenAddressInterface(
        mint,
        owner.publicKey,
    );
    if (!tokenAccount.equals(expectedAta)) {
        throw new Error(
            `Token account mismatch. Expected ${expectedAta.toBase58()}, got ${tokenAccount.toBase58()}`,
        );
    }

    const mintInterface = await getMintInterface(rpc, mint);
    const batches = await createRevokeInterfaceInstructions(
        rpc,
        payer.publicKey,
        mint,
        tokenAccount,
        owner.publicKey,
        mintInterface.mint.decimals,
    );

    const additionalSigners = dedupeSigner(payer, [owner]);
    const { rest: loads, last: revokeIxs } = sliceLast(batches);

    await Promise.all(
        loads.map(async ixs => {
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(
                ixs,
                payer,
                blockhash,
                additionalSigners,
            );
            return sendAndConfirmTx(rpc, tx, confirmOptions);
        }),
    );

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(revokeIxs, payer, blockhash, additionalSigners);
    return sendAndConfirmTx(rpc, tx, confirmOptions);
}

/**
 * Build instruction batches for revoking delegation on a light-token ATA.
 *
 * Returns `TransactionInstruction[][]`. Send [0..n-2] in parallel, then [n-1].
 *
 * @param rpc          RPC connection
 * @param payer        Fee payer public key
 * @param mint         Mint address
 * @param tokenAccount Light-token ATA address
 * @param owner        Owner public key
 * @param decimals     Token decimals
 * @returns Instruction batches
 */
export async function createRevokeInterfaceInstructions(
    rpc: Rpc,
    payer: PublicKey,
    mint: PublicKey,
    tokenAccount: PublicKey,
    owner: PublicKey,
    decimals: number,
): Promise<TransactionInstruction[][]> {
    assertBetaEnabled();

    const accountInterface = await _getAtaInterface(
        rpc,
        tokenAccount,
        owner,
        mint,
    );

    checkNotFrozen(accountInterface, 'revoke');

    const internalBatches = await _buildLoadBatches(
        rpc,
        payer,
        accountInterface,
        undefined,
        false,
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

export { sliceLast } from './slice-last';
export {
    createLightTokenApproveInstruction,
    createLightTokenRevokeInstruction,
} from '../instructions/approve-revoke';

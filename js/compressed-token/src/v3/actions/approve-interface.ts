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
 * Approve a delegate for an associated token account.
 *
 * Supports light-token, SPL, and Token-2022 mints. For light-token mints,
 * loads cold accounts if needed before sending the approve instruction.
 *
 * @param rpc            RPC connection
 * @param payer          Fee payer (signer)
 * @param tokenAccount   ATA address
 * @param mint           Mint address
 * @param delegate       Delegate to approve
 * @param amount         Amount to delegate
 * @param owner          Owner of the token account (signer)
 * @param confirmOptions Optional confirm options
 * @param programId      Token program ID (default: LIGHT_TOKEN_PROGRAM_ID)
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
    programId: PublicKey = LIGHT_TOKEN_PROGRAM_ID,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    const expectedAta = getAssociatedTokenAddressInterface(
        mint,
        owner.publicKey,
        false,
        programId,
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
        programId,
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
        isSplOrT22 ? programId : undefined,
    );

    checkNotFrozen(accountInterface, 'approve');

    if (isSplOrT22) {
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
 * Revoke delegation for an associated token account.
 *
 * Supports light-token, SPL, and Token-2022 mints. For light-token mints,
 * loads cold accounts if needed before sending the revoke instruction.
 *
 * @param rpc            RPC connection
 * @param payer          Fee payer (signer)
 * @param tokenAccount   ATA address
 * @param mint           Mint address
 * @param owner          Owner of the token account (signer)
 * @param confirmOptions Optional confirm options
 * @param programId      Token program ID (default: LIGHT_TOKEN_PROGRAM_ID)
 * @returns Transaction signature
 */
export async function revokeInterface(
    rpc: Rpc,
    payer: Signer,
    tokenAccount: PublicKey,
    mint: PublicKey,
    owner: Signer,
    confirmOptions?: ConfirmOptions,
    programId: PublicKey = LIGHT_TOKEN_PROGRAM_ID,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    const expectedAta = getAssociatedTokenAddressInterface(
        mint,
        owner.publicKey,
        false,
        programId,
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
        programId,
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
        isSplOrT22 ? programId : undefined,
    );

    checkNotFrozen(accountInterface, 'revoke');

    if (isSplOrT22) {
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

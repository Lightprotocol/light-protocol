import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionInstruction,
    TransactionSignature,
} from '@solana/web3.js';
import {
    Rpc,
    buildAndSignTx,
    sendAndConfirmTx,
    dedupeSigner,
    assertBetaEnabled,
} from '@lightprotocol/stateless.js';
import { sliceLast } from './transfer-interface';
import { getMint, TokenAccountNotFoundError } from '@solana/spl-token';
import BN from 'bn.js';
import { createUnwrapInstruction } from '../instructions/unwrap';
import {
    getSplInterfaceInfos,
    SplInterfaceInfo,
} from '../../utils/get-token-pool-infos';
import { getAssociatedTokenAddressInterface } from '../get-associated-token-address-interface';
import {
    getAtaInterface as _getAtaInterface,
    assertNotFrozen,
    type AccountInterface,
} from '../get-account-interface';
import { _buildLoadBatches, calculateLoadBatchComputeUnits } from './load-ata';
import { InterfaceOptions } from './transfer-interface';
import { assertTransactionSizeWithinLimit } from '../utils/estimate-tx-size';
import { ERR_NO_LIGHT_TOKEN_BALANCE_UNWRAP } from '../errors';

/**
 * Build instruction batches for unwrapping light-tokens to SPL/T22 tokens.
 *
 * Returns `TransactionInstruction[][]` with the same shape as
 * `createLoadAtaInstructions` and `createTransferInterfaceInstructions`:
 * each inner array is one transaction. Load batches (if any) come first,
 * followed by one final unwrap transaction.
 *
 * Uses amount-aware input selection: only loads the cold inputs needed to
 * cover the unwrap amount (plus padding to fill a single proof batch).
 *
 * @param rpc               RPC connection
 * @param destination       Destination SPL/T22 token account (must exist)
 * @param owner             Owner of the light-token
 * @param mint              Mint address
 * @param amount            Amount to unwrap (defaults to full balance)
 * @param payer             Fee payer (defaults to owner)
 * @param splInterfaceInfo  Optional: SPL interface info
 * @param maxTopUp          Optional: cap on rent top-up (units of 1k lamports; default no cap)
 * @param interfaceOptions  Optional: interface options for load
 * @param wrap              Whether to use unified (wrap) mode for loading.
 *                          Default false.
 * @returns Instruction batches - each inner array is one transaction
 */
export async function createUnwrapInstructions(
    rpc: Rpc,
    destination: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    amount?: number | bigint | BN,
    payer?: PublicKey,
    splInterfaceInfo?: SplInterfaceInfo,
    maxTopUp?: number,
    interfaceOptions?: InterfaceOptions,
    wrap = false,
): Promise<TransactionInstruction[][]> {
    assertBetaEnabled();

    payer ??= owner;

    // 1. Resolve SPL interface info
    let resolvedSplInterfaceInfo = splInterfaceInfo;
    if (!resolvedSplInterfaceInfo) {
        const splInterfaceInfos = await getSplInterfaceInfos(rpc, mint);
        resolvedSplInterfaceInfo = splInterfaceInfos.find(
            info => info.isInitialized,
        );

        if (!resolvedSplInterfaceInfo) {
            throw new Error(
                `No initialized SPL interface found for mint: ${mint.toBase58()}. ` +
                    `Please create an SPL interface via createSplInterface().`,
            );
        }
    }

    // 2. Check destination exists
    const destAtaInfo = await rpc.getAccountInfo(destination);
    if (!destAtaInfo) {
        throw new Error(
            `Destination account does not exist: ${destination.toBase58()}. ` +
                `Create it first using getOrCreateAssociatedTokenAccount or createAssociatedTokenAccountIdempotentInstruction.`,
        );
    }

    // 3. Derive light-token associated token account and get account interface
    const ctokenAta = getAssociatedTokenAddressInterface(mint, owner);

    let accountInterface: AccountInterface;
    try {
        accountInterface = await _getAtaInterface(
            rpc,
            ctokenAta,
            owner,
            mint,
            undefined,
            undefined,
            wrap,
        );
    } catch (error) {
        if (error instanceof TokenAccountNotFoundError) {
            throw new Error(ERR_NO_LIGHT_TOKEN_BALANCE_UNWRAP);
        }
        throw error;
    }

    assertNotFrozen(accountInterface, 'unwrap');

    const totalBalance = accountInterface.parsed.amount;
    if (totalBalance === BigInt(0)) {
        throw new Error(ERR_NO_LIGHT_TOKEN_BALANCE_UNWRAP);
    }

    const unwrapAmount =
        amount != null ? BigInt(amount.toString()) : totalBalance;

    if (unwrapAmount > totalBalance) {
        throw new Error(
            `Insufficient light-token balance. Requested: ${unwrapAmount}, Available: ${totalBalance}`,
        );
    }

    // 4. Build load batches with amount-aware selection.
    // When amount is specified, pass it as targetAmount for selective loading.
    // When amount is undefined (unwrap all), pass undefined to load everything.
    const internalBatches = await _buildLoadBatches(
        rpc,
        payer,
        accountInterface,
        interfaceOptions,
        wrap,
        ctokenAta,
        amount !== undefined ? unwrapAmount : undefined,
    );

    // 5. Get mint decimals
    const mintInfo = await getMint(
        rpc,
        mint,
        undefined,
        resolvedSplInterfaceInfo.tokenProgram,
    );

    // 6. Build unwrap instruction
    const ix = createUnwrapInstruction(
        ctokenAta,
        destination,
        owner,
        mint,
        unwrapAmount,
        resolvedSplInterfaceInfo,
        mintInfo.decimals,
        payer,
        maxTopUp,
    );

    const unwrapBatch: TransactionInstruction[] = [
        ComputeBudgetProgram.setComputeUnitLimit({ units: 200_000 }),
        ix,
    ];

    // 7. Assemble: load batches with CU budgets + unwrap batch
    const numSigners = payer.equals(owner) ? 1 : 2;
    const result: TransactionInstruction[][] = [];

    for (const batch of internalBatches) {
        const cu = calculateLoadBatchComputeUnits(batch);
        const txIxs = [
            ComputeBudgetProgram.setComputeUnitLimit({ units: cu }),
            ...batch.instructions,
        ];
        assertTransactionSizeWithinLimit(txIxs, numSigners, 'Unwrap batch');
        result.push(txIxs);
    }

    assertTransactionSizeWithinLimit(unwrapBatch, numSigners, 'Unwrap batch');
    result.push(unwrapBatch);

    return result;
}

/**
 * Unwrap light-tokens to SPL tokens.
 *
 * Loads cold state to the light-token associated token account, then unwraps to the destination
 * SPL/T22 token account. Uses `createUnwrapInstructions` internally.
 *
 * @param rpc                RPC connection
 * @param payer              Fee payer
 * @param destination        Destination SPL/T22 token account
 * @param owner              Owner of the light-token (signer)
 * @param mint               Mint address
 * @param amount             Amount to unwrap (defaults to all)
 * @param splInterfaceInfo   SPL interface info
 * @param maxTopUp           Optional: cap on rent top-up (units of 1k lamports; default no cap)
 * @param confirmOptions     Confirm options
 *
 * @returns Transaction signature of the unwrap transaction
 */
export async function unwrap(
    rpc: Rpc,
    payer: Signer,
    destination: PublicKey,
    owner: Signer,
    mint: PublicKey,
    amount?: number | bigint | BN,
    splInterfaceInfo?: SplInterfaceInfo,
    maxTopUp?: number,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const batches = await createUnwrapInstructions(
        rpc,
        destination,
        owner.publicKey,
        mint,
        amount,
        payer.publicKey,
        splInterfaceInfo,
        maxTopUp,
    );

    const additionalSigners = dedupeSigner(payer, [owner]);
    const { rest: loads, last: unwrapIxs } = sliceLast(batches);

    await Promise.all(
        loads.map(async ixs => {
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(ixs, payer, blockhash, additionalSigners);
            return sendAndConfirmTx(rpc, tx, confirmOptions);
        }),
    );

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(unwrapIxs, payer, blockhash, additionalSigners);
    return sendAndConfirmTx(rpc, tx, confirmOptions);
}

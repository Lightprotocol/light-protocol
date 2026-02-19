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
    LIGHT_TOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import {
    TokenAccountNotFoundError,
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    createTransferCheckedInstruction,
    getMint,
} from '@solana/spl-token';
import BN from 'bn.js';
import { createLightTokenTransferInstruction } from '../instructions/transfer-interface';
import { createAssociatedTokenAccountInterfaceIdempotentInstruction } from '../instructions/create-ata-interface';
import { getAssociatedTokenAddressInterface } from '../get-associated-token-address-interface';
import { type SplInterfaceInfo } from '../../utils/get-token-pool-infos';
import {
    _buildLoadBatches,
    calculateLoadBatchComputeUnits,
    type InternalLoadBatch,
} from './load-ata';
import {
    getAtaInterface as _getAtaInterface,
    type AccountInterface,
    TokenAccountSourceType,
} from '../get-account-interface';
import { DEFAULT_COMPRESSIBLE_CONFIG } from '../instructions/create-associated-ctoken';
import {
    estimateTransactionSize,
    MAX_TRANSACTION_SIZE,
} from '../utils/estimate-tx-size';

/**
 * Options for interface operations (load, transfer)
 */
export interface InterfaceOptions {
    /** SPL interface infos (fetched if not provided) */
    splInterfaceInfos?: SplInterfaceInfo[];
}

/**
 * Transfer tokens using the c-token interface.
 *
 * High-level action: resolves balances, builds all instructions (load +
 * transfer), signs, and sends. Creates the recipient ATA if it does not exist.
 *
 * For instruction-level control, use `createTransferInterfaceInstructions`.
 *
 * @param rpc             RPC connection
 * @param payer           Fee payer (signer)
 * @param source          Source c-token ATA address
 * @param mint            Mint address
 * @param destination     Recipient wallet public key
 * @param owner           Source owner (signer)
 * @param amount          Amount to transfer
 * @param programId       Token program ID (default: LIGHT_TOKEN_PROGRAM_ID)
 * @param confirmOptions  Optional confirm options
 * @param options         Optional interface options
 * @param wrap            Include SPL/T22 wrapping (default: false)
 * @returns Transaction signature of the transfer transaction
 */
export async function transferInterface(
    rpc: Rpc,
    payer: Signer,
    source: PublicKey,
    mint: PublicKey,
    destination: PublicKey,
    owner: Signer,
    amount: number | bigint | BN,
    programId: PublicKey = LIGHT_TOKEN_PROGRAM_ID,
    confirmOptions?: ConfirmOptions,
    options?: InterfaceOptions,
    wrap = false,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    // Validate source matches owner
    const expectedSource = getAssociatedTokenAddressInterface(
        mint,
        owner.publicKey,
        false,
        programId,
    );
    if (!source.equals(expectedSource)) {
        throw new Error(
            `Source mismatch. Expected ${expectedSource.toBase58()}, got ${source.toBase58()}`,
        );
    }

    const amountBigInt = BigInt(amount.toString());

    // Build all instruction batches. ensureRecipientAta: true (default)
    // includes idempotent ATA creation in the transfer tx -- no extra RPC
    // fetch needed.
    const batches = await createTransferInterfaceInstructions(
        rpc,
        payer.publicKey,
        mint,
        amountBigInt,
        owner.publicKey,
        destination,
        { ...options, wrap, programId, ensureRecipientAta: true },
    );

    const additionalSigners = dedupeSigner(payer, [owner]);
    const { rest: loads, last: transferIxs } = sliceLast(batches);

    // Send load transactions in parallel (if any)
    if (loads.length > 0) {
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
    }

    // Send transfer transaction
    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(transferIxs, payer, blockhash, additionalSigners);

    return sendAndConfirmTx(rpc, tx, confirmOptions);
}

/**
 * Options for createTransferInterfaceInstructions.
 */
export interface TransferOptions extends InterfaceOptions {
    /** Include SPL/T22 wrapping to c-token ATA (unified path). Default: false. */
    wrap?: boolean;
    /** Token program ID. Default: LIGHT_TOKEN_PROGRAM_ID. */
    programId?: PublicKey;
    /**
     * Include an idempotent recipient ATA creation instruction in the
     * transfer transaction. No extra RPC fetch -- uses
     * createAssociatedTokenAccountInterfaceIdempotentInstruction which is
     * a no-op on-chain if the ATA already exists (~200 CU overhead).
     * Default: true.
     */
    ensureRecipientAta?: boolean;
}

/**
 * Splits the last element from an array.
 *
 * Useful for separating load transactions (parallel) from the final transfer
 * transaction (sequential) returned by `createTransferInterfaceInstructions`.
 *
 * @returns `{ rest, last }` where `rest` is everything before the last
 * element and `last` is the last element.
 * @throws if the input array is empty.
 */
export function sliceLast<T>(items: T[]): { rest: T[]; last: T } {
    if (items.length === 0) {
        throw new Error('sliceLast: array must not be empty');
    }
    return { rest: items.slice(0, -1), last: items.at(-1)! };
}

/**
 * Compute units for the transfer transaction (load chunk + transfer).
 * @internal Exported for unit testing.
 */
export function calculateTransferCU(loadBatch: InternalLoadBatch | null): number {
    let cu = 10_000; // c-token transfer base

    if (loadBatch) {
        if (loadBatch.hasAtaCreation) cu += 30_000;
        cu += loadBatch.wrapCount * 50_000;

        if (loadBatch.compressedAccounts.length > 0) {
            // Base cost for Transfer2 CPI chain
            cu += 50_000;
            const needsFullProof = loadBatch.compressedAccounts.some(
                acc => !(acc.compressedAccount.proveByIndex ?? false),
            );
            if (needsFullProof) cu += 100_000;
            for (const acc of loadBatch.compressedAccounts) {
                cu +=
                    (acc.compressedAccount.proveByIndex ?? false)
                        ? 10_000
                        : 30_000;
            }
        }
    }

    cu = Math.ceil(cu * 1.3);
    return Math.max(50_000, Math.min(1_400_000, cu));
}

/**
 * Assert that a batch of instructions fits within the max transaction size.
 * Throws if the estimated size exceeds MAX_TRANSACTION_SIZE.
 */
function assertTxSize(
    instructions: TransactionInstruction[],
    numSigners: number,
): void {
    const size = estimateTransactionSize(instructions, numSigners);
    if (size > MAX_TRANSACTION_SIZE) {
        throw new Error(
            `Batch exceeds max transaction size: ${size} > ${MAX_TRANSACTION_SIZE}. ` +
                `This indicates a bug in batch assembly.`,
        );
    }
}

/**
 * Create instructions for a c-token transfer.
 *
 * Returns `TransactionInstruction[][]` -- an array of transaction instruction
 * arrays. Each inner array is one transaction to sign and send.
 *
 * - All transactions except the last can be sent in parallel (load/decompress).
 * - The last transaction is the transfer and must be sent after all others
 *   confirm.
 * - For a hot sender or <=8 cold inputs, the result is a single-element array.
 *
 * Use `sliceLast` to separate the parallel prefix from the final transfer:
 * ```
 * const batches = await createTransferInterfaceInstructions(...);
 * const { rest, last } = sliceLast(batches);
 * ```
 *
 * When `ensureRecipientAta` is true (the default), an idempotent ATA creation
 * instruction is included in the transfer (last) transaction. No extra RPC
 * fetch -- the instruction is a no-op on-chain if the ATA already exists.
 * Set `ensureRecipientAta: false` if you manage recipient ATAs yourself.
 *
 * All transactions require payer + sender as signers.
 *
 * Hash uniqueness guarantee: all compressed accounts for the sender are
 * fetched once, then partitioned into non-overlapping chunks by tree version.
 * Each hash appears in exactly one batch. This is enforced at runtime by
 * `assertUniqueInputHashes` inside `_buildLoadBatches`.
 *
 * @param rpc       RPC connection
 * @param payer     Fee payer public key
 * @param mint      Mint address
 * @param amount    Amount to transfer
 * @param sender    Sender public key (must sign all transactions)
 * @param recipient Recipient public key
 * @param options   Optional configuration
 * @returns TransactionInstruction[][] -- send [0..n-2] in parallel, then [n-1]
 */
export async function createTransferInterfaceInstructions(
    rpc: Rpc,
    payer: PublicKey,
    mint: PublicKey,
    amount: number | bigint | BN,
    sender: PublicKey,
    recipient: PublicKey,
    options?: TransferOptions,
): Promise<TransactionInstruction[][]> {
    assertBetaEnabled();

    const amountBigInt = BigInt(amount.toString());

    if (amountBigInt <= BigInt(0)) {
        throw new Error('Transfer amount must be greater than zero.');
    }

    const {
        wrap = false,
        programId = LIGHT_TOKEN_PROGRAM_ID,
        ensureRecipientAta = true,
        ...interfaceOptions
    } = options ?? {};

    // Validate recipient is a wallet (on-curve), not an ATA or PDA.
    // Passing an ATA here would derive an ATA-of-ATA and lose funds.
    if (!PublicKey.isOnCurve(recipient.toBytes())) {
        throw new Error(
            `Recipient must be a wallet public key (on-curve), not a PDA or ATA. ` +
                `Got: ${recipient.toBase58()}`,
        );
    }

    const isSplOrT22 =
        programId.equals(TOKEN_PROGRAM_ID) ||
        programId.equals(TOKEN_2022_PROGRAM_ID);

    // Derive ATAs
    const senderAta = getAssociatedTokenAddressInterface(
        mint,
        sender,
        false,
        programId,
    );
    const recipientAta = getAssociatedTokenAddressInterface(
        mint,
        recipient,
        false,
        programId,
    );

    // Get sender's account state
    let senderInterface: AccountInterface;
    try {
        senderInterface = await _getAtaInterface(
            rpc,
            senderAta,
            sender,
            mint,
            undefined,
            programId.equals(LIGHT_TOKEN_PROGRAM_ID) ? undefined : programId,
            wrap,
        );
    } catch (error) {
        if (error instanceof TokenAccountNotFoundError) {
            throw new Error('Sender has no token accounts for this mint.');
        }
        throw error;
    }

    // Frozen handling: match SPL semantics. Frozen accounts cannot be
    // decompressed or wrapped, but unfrozen accounts can still be used.
    // If the hot account itself is frozen, the on-chain transfer program
    // will reject, so we fail early.
    const senderSources = senderInterface._sources ?? [];
    const hotSourceType =
        isSplOrT22 && !wrap
            ? programId.equals(TOKEN_PROGRAM_ID)
                ? TokenAccountSourceType.Spl
                : TokenAccountSourceType.Token2022
            : TokenAccountSourceType.CTokenHot;
    const hotSource = senderSources.find(s => s.type === hotSourceType);
    if (hotSource?.parsed.isFrozen) {
        throw new Error('Cannot transfer: sender token account is frozen.');
    }

    // Calculate unfrozen balance (frozen accounts are excluded from load batches)
    const unfrozenBalance = senderSources
        .filter(s => !s.parsed.isFrozen)
        .reduce((sum, s) => sum + s.amount, BigInt(0));

    if (unfrozenBalance < amountBigInt) {
        const frozenBalance = senderInterface.parsed.amount - unfrozenBalance;
        const frozenNote =
            frozenBalance > BigInt(0)
                ? ` (${frozenBalance} frozen, not usable)`
                : '';
        throw new Error(
            `Insufficient balance. Required: ${amountBigInt}, ` +
                `Available (unfrozen): ${unfrozenBalance}${frozenNote}`,
        );
    }

    // Build load batches for sender (empty if sender is fully hot).
    // Pass amountBigInt so only needed cold inputs are selected.
    const internalBatches = await _buildLoadBatches(
        rpc,
        payer,
        senderInterface,
        interfaceOptions,
        wrap,
        senderAta,
        amountBigInt,
    );

    // Transfer instruction: dispatch based on program
    let transferIx: TransactionInstruction;
    if (isSplOrT22 && !wrap) {
        const mintInfo = await getMint(rpc, mint, undefined, programId);
        transferIx = createTransferCheckedInstruction(
            senderAta,
            mint,
            recipientAta,
            sender,
            amountBigInt,
            mintInfo.decimals,
            [],
            programId,
        );
    } else {
        transferIx = createLightTokenTransferInstruction(
            senderAta,
            recipientAta,
            sender,
            amountBigInt,
        );
    }

    // Create Recipient ATA idempotently. Optional.
    const recipientAtaIxs: TransactionInstruction[] = [];
    if (ensureRecipientAta) {
        recipientAtaIxs.push(
            createAssociatedTokenAccountInterfaceIdempotentInstruction(
                payer,
                recipientAta,
                recipient,
                mint,
                programId,
                undefined, // associatedTokenProgramId (auto-derived)
                programId.equals(LIGHT_TOKEN_PROGRAM_ID)
                    ? { compressibleConfig: DEFAULT_COMPRESSIBLE_CONFIG }
                    : undefined,
            ),
        );
    }

    // Number of signers for size estimation (payer + sender; may be same key)
    const numSigners = payer.equals(sender) ? 1 : 2;

    // Assemble result: TransactionInstruction[][]
    // Last element is always the transfer tx. Preceding elements are
    // load txs that can be sent in parallel.
    // Load txs include budgeting and ATA creation too.
    if (internalBatches.length === 0) {
        // Sender is hot: single transfer tx
        const cu = calculateTransferCU(null);
        const txIxs = [
            ComputeBudgetProgram.setComputeUnitLimit({ units: cu }),
            ...recipientAtaIxs,
            transferIx,
        ];
        assertTxSize(txIxs, numSigners);
        return [txIxs];
    }

    if (internalBatches.length === 1) {
        // Single load batch: combine with transfer in one tx
        const batch = internalBatches[0];
        const cu = calculateTransferCU(batch);
        const txIxs = [
            ComputeBudgetProgram.setComputeUnitLimit({ units: cu }),
            ...recipientAtaIxs,
            ...batch.instructions,
            transferIx,
        ];
        assertTxSize(txIxs, numSigners);
        return [txIxs];
    }

    // Multiple load batches (>8 compressed inputs):
    // [0..n-2]: load-only (send in parallel)
    // [n-1]: last load chunk + transfer (send after others confirm)
    const result: TransactionInstruction[][] = [];

    for (let i = 0; i < internalBatches.length - 1; i++) {
        const batch = internalBatches[i];
        const cu = calculateLoadBatchComputeUnits(batch);
        const txIxs = [
            ComputeBudgetProgram.setComputeUnitLimit({ units: cu }),
            ...batch.instructions,
        ];
        assertTxSize(txIxs, numSigners);
        result.push(txIxs);
    }

    const lastBatch = internalBatches[internalBatches.length - 1];
    const lastCu = calculateTransferCU(lastBatch);
    const lastTxIxs = [
        ComputeBudgetProgram.setComputeUnitLimit({ units: lastCu }),
        ...recipientAtaIxs,
        ...lastBatch.instructions,
        transferIx,
    ];
    assertTxSize(lastTxIxs, numSigners);
    result.push(lastTxIxs);

    return result;
}

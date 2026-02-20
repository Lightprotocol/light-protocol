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
    rawLoadBatchComputeUnits,
    type InternalLoadBatch,
} from './load-ata';
import {
    getAtaInterface as _getAtaInterface,
    type AccountInterface,
    spendableAmountForAuthority,
    isAuthorityForInterface,
    filterInterfaceForAuthority,
} from '../get-account-interface';
import { DEFAULT_COMPRESSIBLE_CONFIG } from '../instructions/create-associated-ctoken';
import {
    assertTransactionSizeWithinLimit,
    estimateTransactionSize,
    MAX_TRANSACTION_SIZE,
} from '../utils/estimate-tx-size';
import { COLD_SOURCE_TYPES } from '../get-account-interface';

/**
 * Options for interface operations (load, transfer)
 */
export interface InterfaceOptions {
    /** SPL interface infos (fetched if not provided) */
    splInterfaceInfos?: SplInterfaceInfo[];
    /**
     * ATA owner when the signer is the delegate (not the owner).
     * For load: use this owner for getAtaInterface; only sources the delegate
     * can use are included. For transfer: see TransferOptions.owner.
     */
    owner?: PublicKey;
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

    const effectiveOwner = options?.owner ?? owner.publicKey;
    const expectedSource = getAssociatedTokenAddressInterface(
        mint,
        effectiveOwner,
        false,
        programId,
    );
    if (!source.equals(expectedSource)) {
        throw new Error(
            `Source mismatch. Expected ${expectedSource.toBase58()}, got ${source.toBase58()}`,
        );
    }

    const amountBigInt = BigInt(amount.toString());

    const batches = await createTransferInterfaceInstructions(
        rpc,
        payer.publicKey,
        mint,
        amountBigInt,
        owner.publicKey,
        destination,
        {
            ...options,
            wrap,
            programId,
            ensureRecipientAta: true,
            owner: options?.owner,
        },
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
    /**
     * ATA owner when the signer is the delegate (not the owner).
     * Required when transferring as delegate: pass the owner so the SDK
     * can derive the source ATA and validate the signer is the account delegate.
     */
    owner?: PublicKey;
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

/** c-token transfer instruction base CU. */
const TRANSFER_BASE_CU = 10_000;

/**
 * Compute units for the transfer transaction (load chunk + transfer).
 * @internal
 */
export function calculateTransferCU(
    loadBatch: InternalLoadBatch | null,
): number {
    const rawLoadCu = loadBatch ? rawLoadBatchComputeUnits(loadBatch) : 0;
    const cu = Math.ceil((TRANSFER_BASE_CU + rawLoadCu) * 1.3);
    return Math.max(50_000, Math.min(1_400_000, cu));
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
        owner: optionsOwner,
        ...interfaceOptions
    } = options ?? {};

    const effectiveOwner = optionsOwner ?? sender;

    if (!PublicKey.isOnCurve(recipient.toBytes())) {
        throw new Error(
            `Recipient must be a wallet public key (on-curve), not a PDA or ATA. ` +
                `Got: ${recipient.toBase58()}`,
        );
    }

    const isSplOrT22 =
        programId.equals(TOKEN_PROGRAM_ID) ||
        programId.equals(TOKEN_2022_PROGRAM_ID);

    const senderAta = getAssociatedTokenAddressInterface(
        mint,
        effectiveOwner,
        false,
        programId,
    );
    const recipientAta = getAssociatedTokenAddressInterface(
        mint,
        recipient,
        false,
        programId,
    );

    let senderInterface: AccountInterface;
    try {
        senderInterface = await _getAtaInterface(
            rpc,
            senderAta,
            effectiveOwner,
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

    if (senderInterface._anyFrozen) {
        throw new Error(
            'Account is frozen. One or more sources (hot or cold) are frozen; transfer is not allowed.',
        );
    }

    const isDelegate = !effectiveOwner.equals(sender);
    if (isDelegate) {
        if (!isAuthorityForInterface(senderInterface, sender)) {
            throw new Error(
                'Signer is not the owner or a delegate of the sender account.',
            );
        }
        const spendable = spendableAmountForAuthority(senderInterface, sender);
        if (amountBigInt > spendable) {
            throw new Error(
                `Insufficient delegated balance. Required: ${amountBigInt}, Available (delegate): ${spendable}`,
            );
        }
        senderInterface = filterInterfaceForAuthority(senderInterface, sender);
    } else {
        if (senderInterface.parsed.amount < amountBigInt) {
            throw new Error(
                `Insufficient balance. Required: ${amountBigInt}, Available: ${senderInterface.parsed.amount}`,
            );
        }
    }

    const internalBatches = await _buildLoadBatches(
        rpc,
        payer,
        senderInterface,
        interfaceOptions,
        wrap,
        senderAta,
        amountBigInt,
        sender,
    );

    // For delegate transfers that need cold source loading: approve-style
    // compressed accounts (no CompressedOnly TLV) will NOT have their delegate
    // applied to the hot ATA during decompress. Only compress-and-close
    // accounts (with CompressedOnly TLV) carry over delegate state.
    if (isDelegate && internalBatches.length > 0) {
        const sources = senderInterface._sources ?? [];
        const hasApproveStyleCold = sources.some(
            s =>
                COLD_SOURCE_TYPES.has(s.type) &&
                s.parsed.delegate !== null &&
                s.parsed.delegate.equals(sender) &&
                (!s.parsed.tlvData || s.parsed.tlvData.length === 0),
        );
        if (hasApproveStyleCold) {
            throw new Error(
                'Delegate transfer requires loading cold sources that were delegated ' +
                    'via approve (no CompressedOnly TLV). Decompress will not carry ' +
                    'the delegate to the hot ATA. Load as owner first, then approve ' +
                    'the delegate on the hot ATA.',
            );
        }
    }

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
        assertTransactionSizeWithinLimit(txIxs, numSigners, 'Batch');
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
        assertTransactionSizeWithinLimit(txIxs, numSigners, 'Batch');
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
        assertTransactionSizeWithinLimit(txIxs, numSigners, 'Batch');
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
    assertTransactionSizeWithinLimit(lastTxIxs, numSigners, 'Batch');
    result.push(lastTxIxs);

    return result;
}

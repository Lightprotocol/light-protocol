import {
    Rpc,
    LIGHT_TOKEN_PROGRAM_ID,
    buildAndSignTx,
    sendAndConfirmTx,
    dedupeSigner,
    ParsedTokenAccount,
    bn,
    assertBetaEnabled,
} from '@lightprotocol/stateless.js';
import { assertV2Only } from '../assert-v2-only';
import {
    PublicKey,
    TransactionInstruction,
    Signer,
    TransactionSignature,
    ConfirmOptions,
    ComputeBudgetProgram,
} from '@solana/web3.js';
import {
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    getAssociatedTokenAddressSync,
    createAssociatedTokenAccountIdempotentInstruction,
    getMint,
    TokenAccountNotFoundError,
} from '@solana/spl-token';
import {
    AccountInterface,
    getAtaInterface as _getAtaInterface,
    TokenAccountSource,
    TokenAccountSourceType,
    isAuthorityForInterface,
    filterInterfaceForAuthority,
} from '../get-account-interface';
import { getAssociatedTokenAddressInterface } from '../get-associated-token-address-interface';
import { createAssociatedTokenAccountInterfaceIdempotentInstruction } from '../instructions/create-ata-interface';
import { createWrapInstruction } from '../instructions/wrap';
import { createDecompressInterfaceInstruction } from '../instructions/create-decompress-interface-instruction';
import {
    getSplInterfaceInfos,
    SplInterfaceInfo,
} from '../../utils/get-token-pool-infos';
import { getAtaProgramId, checkAtaAddress, AtaType } from '../ata-utils';
import { InterfaceOptions } from './transfer-interface';

/**
 * Maximum input compressed accounts per Transfer2 instruction.
 * Defined in programs/compressed-token/program/src/shared/cpi_bytes_size.rs
 */
export const MAX_INPUT_ACCOUNTS = 8;

/** All source types that represent compressed (cold) accounts. */
const COLD_SOURCE_TYPES: ReadonlySet<string> = new Set([
    TokenAccountSourceType.CTokenCold,
    TokenAccountSourceType.SplCold,
    TokenAccountSourceType.Token2022Cold,
]);

/**
 * Split an array into chunks of specified size.
 * @internal
 */
function chunkArray<T>(array: T[], chunkSize: number): T[][] {
    const chunks: T[][] = [];
    for (let i = 0; i < array.length; i += chunkSize) {
        chunks.push(array.slice(i, i + chunkSize));
    }
    return chunks;
}

/**
 * Select compressed inputs for a target amount.
 *
 * Sorts by amount descending (largest first), accumulates until the target
 * is met, then pads to {@link MAX_INPUT_ACCOUNTS} if possible within a
 * single batch.
 *
 * - If the amount is covered by N <= 8 inputs, returns min(8, total) inputs.
 * - If more than 8 inputs are needed, returns exactly as many as required
 *   (no padding beyond the amount-needed count).
 * - Returns [] when `neededAmount <= 0` or `accounts` is empty.
 *
 * @param accounts      Cold compressed token accounts available for loading.
 * @param neededAmount  Amount that must be covered by selected inputs.
 * @returns Subset of `accounts`, sorted largest-first.
 */
export function selectInputsForAmount(
    accounts: ParsedTokenAccount[],
    neededAmount: bigint,
): ParsedTokenAccount[] {
    if (accounts.length === 0 || neededAmount <= BigInt(0)) return [];

    const sorted = [...accounts].sort((a, b) => {
        const amtA = BigInt(a.parsed.amount.toString());
        const amtB = BigInt(b.parsed.amount.toString());
        if (amtB > amtA) return 1;
        if (amtB < amtA) return -1;
        return 0;
    });

    let accumulated = BigInt(0);
    let countNeeded = 0;
    for (const acc of sorted) {
        countNeeded++;
        accumulated += BigInt(acc.parsed.amount.toString());
        if (accumulated >= neededAmount) break;
    }

    // Pad to MAX_INPUT_ACCOUNTS if within a single batch
    const selectCount = Math.min(
        Math.max(countNeeded, MAX_INPUT_ACCOUNTS),
        sorted.length,
    );

    return sorted.slice(0, selectCount);
}

/**
 * Verify no compressed account hash appears in more than one chunk.
 * Prevents double-spending of inputs across parallel batches.
 * @internal
 */
function assertUniqueInputHashes(chunks: ParsedTokenAccount[][]): void {
    const seen = new Set<string>();
    for (const chunk of chunks) {
        for (const acc of chunk) {
            const hashStr = acc.compressedAccount.hash.toString();
            if (seen.has(hashStr)) {
                throw new Error(
                    `Duplicate compressed account hash across chunks: ${hashStr}. ` +
                        `Each compressed account must appear in exactly one chunk.`,
                );
            }
            seen.add(hashStr);
        }
    }
}

/** @internal */
export function getCompressedTokenAccountsFromAtaSources(
    sources: TokenAccountSource[],
): ParsedTokenAccount[] {
    const coldTypes = new Set<TokenAccountSource['type']>([
        TokenAccountSourceType.CTokenCold,
        TokenAccountSourceType.SplCold,
        TokenAccountSourceType.Token2022Cold,
    ]);

    return sources
        .filter(source => source.loadContext !== undefined)
        .filter(source => coldTypes.has(source.type))
        .filter(source => !source.parsed.isFrozen)
        .map(source => {
            const fullData = source.accountInfo.data;
            const discriminatorBytes = fullData.subarray(
                0,
                Math.min(8, fullData.length),
            );
            const accountDataBytes =
                fullData.length > 8 ? fullData.subarray(8) : Buffer.alloc(0);

            const compressedAccount = {
                treeInfo: source.loadContext!.treeInfo,
                hash: source.loadContext!.hash,
                leafIndex: source.loadContext!.leafIndex,
                proveByIndex: source.loadContext!.proveByIndex,
                owner: source.accountInfo.owner,
                lamports: bn(source.accountInfo.lamports),
                address: null,
                data:
                    fullData.length === 0
                        ? null
                        : {
                              discriminator: Array.from(discriminatorBytes),
                              data: Buffer.from(accountDataBytes),
                              dataHash: new Array(32).fill(0),
                          },
                readOnly: false,
            };

            const state = !source.parsed.isInitialized
                ? 0
                : source.parsed.isFrozen
                  ? 2
                  : 1;

            return {
                compressedAccount: compressedAccount as any,
                parsed: {
                    mint: source.parsed.mint,
                    owner: source.parsed.owner,
                    amount: bn(source.parsed.amount.toString()),
                    delegate: source.parsed.delegate,
                    state,
                    tlv:
                        source.parsed.tlvData.length > 0
                            ? source.parsed.tlvData
                            : null,
                },
            } satisfies ParsedTokenAccount;
        });
}

// Re-export types moved to instructions
export {
    ParsedAccountInfoInterface,
    CompressibleAccountInput,
    PackedCompressedAccount,
    CompressibleLoadParams,
    LoadResult,
    createLoadAccountsParams,
    calculateCompressibleLoadComputeUnits,
} from '../instructions/create-load-accounts-params';

// Re-export AtaType for backwards compatibility
export { AtaType } from '../ata-utils';

/**
 * Create instruction batches for loading token balances into an ATA.
 * Handles >8 compressed accounts by returning multiple transaction batches.
 *
 * IMPORTANT: Each batch must be sent as a SEPARATE transaction because
 * multiple decompress instructions in one transaction will have invalid proofs
 * (the Merkle tree root changes after each decompress).
 *
 * @param rpc               RPC connection
 * @param ata               Associated token address (SPL, T22, or c-token)
 * @param owner             Owner public key
 * @param mint              Mint public key
 * @param payer             Fee payer public key (defaults to owner)
 * @param interfaceOptions  Optional interface options
 * @param wrap              Unified mode: wrap SPL/T22 to c-token (default: false)
 * @returns Instruction batches - each inner array is one transaction
 */
export async function createLoadAtaInstructions(
    rpc: Rpc,
    ata: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    payer?: PublicKey,
    interfaceOptions?: InterfaceOptions,
    wrap = false,
): Promise<TransactionInstruction[][]> {
    assertBetaEnabled();
    payer ??= owner;

    const effectiveOwner = interfaceOptions?.owner ?? owner;

    let accountInterface: AccountInterface;
    try {
        accountInterface = await _getAtaInterface(
            rpc,
            ata,
            effectiveOwner,
            mint,
            undefined,
            undefined,
            wrap,
        );
    } catch (e) {
        if (e instanceof TokenAccountNotFoundError) {
            return [];
        }
        throw e;
    }

    if (accountInterface._anyFrozen) {
        throw new Error(
            'Account is frozen. One or more sources (hot or cold) are frozen; load is not allowed.',
        );
    }

    const isDelegate = !effectiveOwner.equals(owner);
    if (isDelegate) {
        if (!isAuthorityForInterface(accountInterface, owner)) {
            throw new Error(
                'Signer is not the owner or a delegate of the account.',
            );
        }
        accountInterface = filterInterfaceForAuthority(accountInterface, owner);
        if (
            (accountInterface._sources?.length ?? 0) === 0 ||
            accountInterface.parsed.amount === BigInt(0)
        ) {
            return [];
        }
    }

    const internalBatches = await _buildLoadBatches(
        rpc,
        payer,
        accountInterface,
        interfaceOptions,
        wrap,
        ata,
        undefined,
        owner,
    );

    return internalBatches.map(batch => batch.instructions);
}

/**
 * Internal batch structure for loadAta parallel sending.
 * @internal Exported for use by createTransferInterfaceInstructions.
 */
export interface InternalLoadBatch {
    instructions: TransactionInstruction[];
    compressedAccounts: ParsedTokenAccount[];
    wrapCount: number;
    hasAtaCreation: boolean;
}

/**
 * Calculate compute units for a load batch with 30% buffer.
 *
 * Heuristics:
 * - ATA creation: ~30k CU
 * - Wrap operation: ~50k CU each
 * - Decompress base cost (CPI overhead, hash computation): ~50k CU
 * - Full proof verification (when any input is NOT proveByIndex): ~100k CU
 * - Per compressed account: ~10k (proveByIndex) or ~30k (full proof) CU
 * @internal
 */
export function calculateLoadBatchComputeUnits(
    batch: InternalLoadBatch,
): number {
    let cu = 0;

    if (batch.hasAtaCreation) {
        cu += 30_000;
    }

    cu += batch.wrapCount * 50_000;

    if (batch.compressedAccounts.length > 0) {
        // Base cost for Transfer2 CPI chain (cToken -> system -> account-compression)
        cu += 50_000;

        const needsFullProof = batch.compressedAccounts.some(
            acc => !(acc.compressedAccount.proveByIndex ?? false),
        );
        if (needsFullProof) {
            cu += 100_000;
        }
        for (const acc of batch.compressedAccounts) {
            const proveByIndex = acc.compressedAccount.proveByIndex ?? false;
            cu += proveByIndex ? 10_000 : 30_000;
        }
    }

    // 30% buffer
    cu = Math.ceil(cu * 1.3);

    return Math.max(50_000, Math.min(1_400_000, cu));
}

/**
 * Build load instruction batches for parallel sending.
 *
 * Returns one or more batches:
 * - Batch 0: setup (ATA creation, wraps) + first decompress chunk
 * - Batch 1..N: idempotent ATA creation + decompress chunk 1..N
 *
 * Each batch is independent and can be sent in parallel. Idempotent ATA
 * creation is included in every batch so they can land in any order.
 * @internal
 */
export async function _buildLoadBatches(
    rpc: Rpc,
    payer: PublicKey,
    ata: AccountInterface,
    options: InterfaceOptions | undefined,
    wrap: boolean,
    targetAta: PublicKey,
    targetAmount?: bigint,
    authority?: PublicKey,
): Promise<InternalLoadBatch[]> {
    if (!ata._isAta || !ata._owner || !ata._mint) {
        throw new Error(
            'AccountInterface must be from getAtaInterface (requires _isAta, _owner, _mint)',
        );
    }

    if (ata._anyFrozen) {
        throw new Error(
            'Account is frozen. One or more sources (hot or cold) are frozen; load is not allowed.',
        );
    }

    const owner = ata._owner;
    const mint = ata._mint;
    const sources = ata._sources ?? [];

    const allCompressedAccounts =
        getCompressedTokenAccountsFromAtaSources(sources);

    // Derive addresses
    const ctokenAtaAddress = getAssociatedTokenAddressInterface(mint, owner);
    const splAta = getAssociatedTokenAddressSync(
        mint,
        owner,
        false,
        TOKEN_PROGRAM_ID,
        getAtaProgramId(TOKEN_PROGRAM_ID),
    );
    const t22Ata = getAssociatedTokenAddressSync(
        mint,
        owner,
        false,
        TOKEN_2022_PROGRAM_ID,
        getAtaProgramId(TOKEN_2022_PROGRAM_ID),
    );

    // Validate target ATA type
    let ataType: AtaType = 'ctoken';
    const validation = checkAtaAddress(targetAta, mint, owner);
    ataType = validation.type;
    if (wrap && ataType !== 'ctoken') {
        throw new Error(
            `For wrap=true, targetAta must be c-token ATA. Got ${ataType} ATA.`,
        );
    }

    // Check sources for balances (skip frozen for wrappable/decompressible sources)
    const splSource = sources.find(s => s.type === 'spl' && !s.parsed.isFrozen);
    const t22Source = sources.find(
        s => s.type === 'token2022' && !s.parsed.isFrozen,
    );
    const ctokenHotSource = sources.find(
        s => s.type === 'ctoken-hot' && !s.parsed.isFrozen,
    );
    const coldSources = sources.filter(
        s => COLD_SOURCE_TYPES.has(s.type) && !s.parsed.isFrozen,
    );

    const splBalance = splSource?.amount ?? BigInt(0);
    const t22Balance = t22Source?.amount ?? BigInt(0);
    const coldBalance = coldSources.reduce(
        (sum, s) => sum + s.amount,
        BigInt(0),
    );

    if (
        splBalance === BigInt(0) &&
        t22Balance === BigInt(0) &&
        coldBalance === BigInt(0)
    ) {
        return [];
    }

    // Get SPL interface info if needed
    let splInterfaceInfo: SplInterfaceInfo | undefined;
    const needsSplInfo =
        wrap ||
        ataType === 'spl' ||
        ataType === 'token2022' ||
        splBalance > BigInt(0) ||
        t22Balance > BigInt(0);
    let decimals = 0;
    if (needsSplInfo) {
        try {
            const splInterfaceInfos =
                options?.splInterfaceInfos ??
                (await getSplInterfaceInfos(rpc, mint));
            splInterfaceInfo = splInterfaceInfos.find(
                (info: SplInterfaceInfo) => info.isInitialized,
            );
            if (splInterfaceInfo) {
                const mintInfo = await getMint(
                    rpc,
                    mint,
                    undefined,
                    splInterfaceInfo.tokenProgram,
                );
                decimals = mintInfo.decimals;
            }
        } catch (e) {
            if (splBalance > BigInt(0) || t22Balance > BigInt(0)) {
                throw e;
            }
        }
    }

    // Build setup instructions (ATA creation + wraps)
    const setupInstructions: TransactionInstruction[] = [];
    let wrapCount = 0;
    let needsAtaCreation = false;

    // Determine decompress target based on mode
    let decompressTarget: PublicKey = ctokenAtaAddress;
    let decompressSplInfo: SplInterfaceInfo | undefined;
    let canDecompress = false;

    if (wrap) {
        decompressTarget = ctokenAtaAddress;
        decompressSplInfo = undefined;
        canDecompress = true;

        if (!ctokenHotSource) {
            needsAtaCreation = true;
            setupInstructions.push(
                createAssociatedTokenAccountInterfaceIdempotentInstruction(
                    payer,
                    ctokenAtaAddress,
                    owner,
                    mint,
                    LIGHT_TOKEN_PROGRAM_ID,
                ),
            );
        }

        if (splBalance > BigInt(0) && splInterfaceInfo) {
            setupInstructions.push(
                createWrapInstruction(
                    splAta,
                    ctokenAtaAddress,
                    owner,
                    mint,
                    splBalance,
                    splInterfaceInfo,
                    decimals,
                    payer,
                ),
            );
            wrapCount++;
        }

        if (t22Balance > BigInt(0) && splInterfaceInfo) {
            setupInstructions.push(
                createWrapInstruction(
                    t22Ata,
                    ctokenAtaAddress,
                    owner,
                    mint,
                    t22Balance,
                    splInterfaceInfo,
                    decimals,
                    payer,
                ),
            );
            wrapCount++;
        }
    } else {
        if (ataType === 'ctoken') {
            decompressTarget = ctokenAtaAddress;
            decompressSplInfo = undefined;
            canDecompress = true;
            if (!ctokenHotSource) {
                needsAtaCreation = true;
                setupInstructions.push(
                    createAssociatedTokenAccountInterfaceIdempotentInstruction(
                        payer,
                        ctokenAtaAddress,
                        owner,
                        mint,
                        LIGHT_TOKEN_PROGRAM_ID,
                    ),
                );
            }
        } else if (ataType === 'spl' && splInterfaceInfo) {
            decompressTarget = splAta;
            decompressSplInfo = splInterfaceInfo;
            canDecompress = true;
            if (!splSource) {
                needsAtaCreation = true;
                setupInstructions.push(
                    createAssociatedTokenAccountIdempotentInstruction(
                        payer,
                        splAta,
                        owner,
                        mint,
                        TOKEN_PROGRAM_ID,
                    ),
                );
            }
        } else if (ataType === 'token2022' && splInterfaceInfo) {
            decompressTarget = t22Ata;
            decompressSplInfo = splInterfaceInfo;
            canDecompress = true;
            if (!t22Source) {
                needsAtaCreation = true;
                setupInstructions.push(
                    createAssociatedTokenAccountIdempotentInstruction(
                        payer,
                        t22Ata,
                        owner,
                        mint,
                        TOKEN_2022_PROGRAM_ID,
                    ),
                );
            }
        }
    }

    // Amount-aware input selection: when targetAmount is provided, only
    // load the cold inputs needed to cover the transfer/unwrap amount.
    // When targetAmount is undefined (e.g. loadAta), load everything.
    let accountsToLoad = allCompressedAccounts;

    if (
        targetAmount !== undefined &&
        canDecompress &&
        allCompressedAccounts.length > 0
    ) {
        const hotBalance = ctokenHotSource?.amount ?? BigInt(0);
        let effectiveHotAfterSetup: bigint;

        if (wrap) {
            effectiveHotAfterSetup = hotBalance + splBalance + t22Balance;
        } else if (ataType === 'ctoken') {
            effectiveHotAfterSetup = hotBalance;
        } else if (ataType === 'spl') {
            effectiveHotAfterSetup = splBalance;
        } else {
            // token2022
            effectiveHotAfterSetup = t22Balance;
        }

        const neededFromCold =
            targetAmount > effectiveHotAfterSetup
                ? targetAmount - effectiveHotAfterSetup
                : BigInt(0);

        if (neededFromCold === BigInt(0)) {
            accountsToLoad = [];
        } else {
            accountsToLoad = selectInputsForAmount(
                allCompressedAccounts,
                neededFromCold,
            );
        }
    }

    // If no cold accounts to decompress, return just the setup batch
    if (!canDecompress || accountsToLoad.length === 0) {
        if (setupInstructions.length === 0) return [];
        return [
            {
                instructions: setupInstructions,
                compressedAccounts: [],
                wrapCount,
                hasAtaCreation: needsAtaCreation,
            },
        ];
    }

    // V2-only: reject V1 inputs early
    assertV2Only(accountsToLoad);

    // Chunk into non-overlapping groups of MAX_INPUT_ACCOUNTS and verify uniqueness
    const chunks = chunkArray(accountsToLoad, MAX_INPUT_ACCOUNTS);
    assertUniqueInputHashes(chunks);

    // Get proofs for all chunks in parallel
    const proofs = await Promise.all(
        chunks.map(async chunk => {
            const proofInputs = chunk.map(acc => ({
                hash: acc.compressedAccount.hash,
                tree: acc.compressedAccount.treeInfo.tree,
                queue: acc.compressedAccount.treeInfo.queue,
            }));
            return rpc.getValidityProofV0(proofInputs);
        }),
    );

    // Build idempotent ATA creation instruction for subsequent batches
    const idempotentAtaIx = (() => {
        if (wrap || ataType === 'ctoken') {
            return createAssociatedTokenAccountInterfaceIdempotentInstruction(
                payer,
                ctokenAtaAddress,
                owner,
                mint,
                LIGHT_TOKEN_PROGRAM_ID,
            );
        } else if (ataType === 'spl') {
            return createAssociatedTokenAccountIdempotentInstruction(
                payer,
                splAta,
                owner,
                mint,
                TOKEN_PROGRAM_ID,
            );
        } else {
            return createAssociatedTokenAccountIdempotentInstruction(
                payer,
                t22Ata,
                owner,
                mint,
                TOKEN_2022_PROGRAM_ID,
            );
        }
    })();

    // Build batches
    const batches: InternalLoadBatch[] = [];

    for (let i = 0; i < chunks.length; i++) {
        const chunk = chunks[i];
        const proof = proofs[i];
        const chunkAmount = chunk.reduce(
            (sum, acc) => sum + BigInt(acc.parsed.amount.toString()),
            BigInt(0),
        );

        const batchIxs: TransactionInstruction[] = [];
        let batchWrapCount = 0;
        let batchHasAtaCreation = false;

        if (i === 0) {
            // First batch includes all setup (ATA creation + wraps)
            batchIxs.push(...setupInstructions);
            batchWrapCount = wrapCount;
            batchHasAtaCreation = needsAtaCreation;
        } else {
            // Subsequent batches: include idempotent ATA creation so
            // batches can land in any order
            batchIxs.push(idempotentAtaIx);
            batchHasAtaCreation = true;
        }

        const authorityForDecompress = authority ?? owner;
        batchIxs.push(
            createDecompressInterfaceInstruction(
                payer,
                chunk,
                decompressTarget,
                chunkAmount,
                proof,
                decompressSplInfo,
                decimals,
                undefined,
                authorityForDecompress,
            ),
        );

        batches.push({
            instructions: batchIxs,
            compressedAccounts: chunk,
            wrapCount: batchWrapCount,
            hasAtaCreation: batchHasAtaCreation,
        });
    }

    return batches;
}

/**
 * Load token balances into an ATA.
 *
 * Behavior depends on `wrap` parameter:
 * - wrap=false (standard): Decompress compressed tokens to the target ATA.
 *   ATA can be SPL (via pool), T22 (via pool), or c-token (direct).
 * - wrap=true (unified): Wrap SPL/T22 + decompress all to c-token ATA.
 *
 * Handles any number of compressed accounts by building per-chunk batches
 * (max 8 inputs per decompress instruction) and sending all batches in
 * parallel. Each batch includes idempotent ATA creation so landing order
 * does not matter.
 *
 * Idempotent: returns null if nothing to load.
 *
 * @param rpc               RPC connection
 * @param ata               Associated token address (SPL, T22, or c-token)
 * @param owner             Owner of the tokens (signer)
 * @param mint              Mint public key
 * @param payer             Fee payer (signer, defaults to owner)
 * @param confirmOptions    Optional confirm options
 * @param interfaceOptions  Optional interface options
 * @param wrap              Unified mode: wrap SPL/T22 to c-token (default: false)
 * @returns Last transaction signature, or null if nothing to load
 */
export async function loadAta(
    rpc: Rpc,
    ata: PublicKey,
    owner: Signer,
    mint: PublicKey,
    payer?: Signer,
    confirmOptions?: ConfirmOptions,
    interfaceOptions?: InterfaceOptions,
    wrap = false,
): Promise<TransactionSignature | null> {
    assertBetaEnabled();

    payer ??= owner;
    const effectiveOwner = interfaceOptions?.owner ?? owner.publicKey;

    let ataInterface: AccountInterface;
    try {
        ataInterface = await _getAtaInterface(
            rpc,
            ata,
            effectiveOwner,
            mint,
            undefined,
            undefined,
            wrap,
        );
    } catch (error) {
        if (error instanceof TokenAccountNotFoundError) {
            return null;
        }
        throw error;
    }

    if (ataInterface._anyFrozen) {
        throw new Error(
            'Account is frozen. One or more sources (hot or cold) are frozen; load is not allowed.',
        );
    }

    const isDelegate = !effectiveOwner.equals(owner.publicKey);
    if (isDelegate) {
        if (!isAuthorityForInterface(ataInterface, owner.publicKey)) {
            throw new Error(
                'Signer is not the owner or a delegate of the account.',
            );
        }
        ataInterface = filterInterfaceForAuthority(
            ataInterface,
            owner.publicKey,
        );
        if (
            (ataInterface._sources?.length ?? 0) === 0 ||
            ataInterface.parsed.amount === BigInt(0)
        ) {
            return null;
        }
    }

    const batches = await _buildLoadBatches(
        rpc,
        payer.publicKey,
        ataInterface,
        interfaceOptions,
        wrap,
        ata,
        undefined,
        owner.publicKey,
    );

    if (batches.length === 0) {
        return null;
    }

    const additionalSigners = dedupeSigner(payer, [owner]);

    // Send all batches in parallel
    const txPromises = batches.map(async batch => {
        const { blockhash } = await rpc.getLatestBlockhash();
        const computeUnits = calculateLoadBatchComputeUnits(batch);

        const tx = buildAndSignTx(
            [
                ComputeBudgetProgram.setComputeUnitLimit({
                    units: computeUnits,
                }),
                ...batch.instructions,
            ],
            payer!,
            blockhash,
            additionalSigners,
        );

        return sendAndConfirmTx(rpc, tx, confirmOptions);
    });

    const results = await Promise.all(txPromises);
    return results[results.length - 1];
}

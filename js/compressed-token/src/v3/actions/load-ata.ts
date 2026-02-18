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
 * Split an array into chunks of specified size
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
 * @param accounts      Cold light-token accounts available for loading.
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

/**
 * Create a single decompress instruction for compressed accounts.
 * Limited to MAX_INPUT_ACCOUNTS (8) accounts per call.
 *
 * @param rpc                RPC connection
 * @param payer              Fee payer
 * @param compressedAccounts Compressed accounts to decompress (max 8)
 * @param destinationAta     Destination associated token account address
 * @param splInterfaceInfo   Optional SPL interface info (for SPL/T22 decompression)
 * @param decimals           Mint decimals
 * @returns Single decompress instruction
 */
async function createDecompressInstructionForAccounts(
    rpc: Rpc,
    payer: PublicKey,
    compressedAccounts: ParsedTokenAccount[],
    destinationAta: PublicKey,
    splInterfaceInfo: SplInterfaceInfo | undefined,
    decimals: number,
): Promise<TransactionInstruction> {
    if (compressedAccounts.length === 0) {
        throw new Error('No compressed accounts provided');
    }
    if (compressedAccounts.length > MAX_INPUT_ACCOUNTS) {
        throw new Error(
            `Too many compressed accounts: ${compressedAccounts.length} > ${MAX_INPUT_ACCOUNTS}. ` +
                `Use createLoadAtaInstructions for >8 accounts.`,
        );
    }

    const amount = compressedAccounts.reduce(
        (sum, acc) => sum + BigInt(acc.parsed.amount.toString()),
        BigInt(0),
    );

    const proof = await rpc.getValidityProofV0(
        compressedAccounts.map(acc => ({
            hash: acc.compressedAccount.hash,
            tree: acc.compressedAccount.treeInfo.tree,
            queue: acc.compressedAccount.treeInfo.queue,
        })),
    );

    return createDecompressInterfaceInstruction(
        payer,
        compressedAccounts,
        destinationAta,
        amount,
        proof,
        splInterfaceInfo,
        decimals,
    );
}

/**
 * Create decompress instructions for all compressed accounts, chunking into multiple
 * instructions if there are more than MAX_INPUT_ACCOUNTS (8).
 *
 * Each instruction handles a distinct set of accounts - no overlap.
 *
 * @param rpc                RPC connection
 * @param payer              Fee payer
 * @param compressedAccounts All compressed accounts to decompress
 * @param destinationAta     Destination associated token account address
 * @param splInterfaceInfo   Optional SPL interface info (for SPL/T22 decompression)
 * @param decimals           Mint decimals
 * @returns Array of decompress instructions (one per chunk of 8 accounts)
 */
/**
 * Create chunked decompress instructions for multiple compressed accounts.
 * For >8 accounts, creates multiple decompress instructions (one per chunk of 8).
 */
async function createChunkedDecompressInstructions(
    rpc: Rpc,
    payer: PublicKey,
    compressedAccounts: ParsedTokenAccount[],
    destinationAta: PublicKey,
    splInterfaceInfo: SplInterfaceInfo | undefined,
    decimals: number,
): Promise<TransactionInstruction[]> {
    if (compressedAccounts.length === 0) {
        return [];
    }

    assertV2Only(compressedAccounts);

    const instructions: TransactionInstruction[] = [];

    // Split accounts into non-overlapping chunks of MAX_INPUT_ACCOUNTS
    const chunks = chunkArray(compressedAccounts, MAX_INPUT_ACCOUNTS);
    assertUniqueInputHashes(chunks);

    // Get separate proofs for each chunk
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

    for (let chunkIdx = 0; chunkIdx < chunks.length; chunkIdx++) {
        const chunk = chunks[chunkIdx];
        const proof = proofs[chunkIdx];

        const chunkAmount = chunk.reduce(
            (sum, acc) => sum + BigInt(acc.parsed.amount.toString()),
            BigInt(0),
        );

        const ix = createDecompressInterfaceInstruction(
            payer,
            chunk,
            destinationAta,
            chunkAmount,
            proof,
            splInterfaceInfo,
            decimals,
        );

        instructions.push(ix);
    }

    return instructions;
}

function getCompressedTokenAccountsFromAtaSources(
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
 * Create instructions to load an associated token account from its AccountInterface.
 *
 * Behavior depends on `wrap` parameter:
 * - wrap=false (standard): Decompress compressed light-tokens to the target associated token account type
 *   (SPL associated token account via interface PDA, T22 associated token account via interface PDA, or light-token associated token account direct)
 * - wrap=true (unified): Wrap SPL/T22 + decompress all to light-token associated token account
 *
 * @param rpc         RPC connection
 * @param payer       Fee payer
 * @param ata         AccountInterface from getAtaInterface (must have _isAta, _owner, _mint)
 * @param options     Optional load options
 * @param wrap        Unified mode: wrap SPL/T22 to light-token (default: false)
 * @param targetAta   Target associated token account address (used for type detection in standard mode)
 * @returns           Array of instructions (empty if nothing to load)
 */
export async function createLoadAtaInstructionsFromInterface(
    rpc: Rpc,
    payer: PublicKey,
    ata: AccountInterface,
    options?: InterfaceOptions,
    wrap = false,
    targetAta?: PublicKey,
): Promise<TransactionInstruction[]> {
    if (!ata._isAta || !ata._owner || !ata._mint) {
        throw new Error(
            'AccountInterface must be from getAtaInterface (requires _isAta, _owner, _mint)',
        );
    }

    const instructions: TransactionInstruction[] = [];
    const owner = ata._owner;
    const mint = ata._mint;
    const sources = ata._sources ?? [];

    // Precompute compressed accounts from cold sources
    const compressedAccountsToCheck =
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

    // Validate and detect target associated token account type
    // If called via createLoadAtaInstructions, validation already happened in getAtaInterface.
    // If called directly, this validates the targetAta is correct.
    let ataType: AtaType = 'ctoken';
    if (targetAta) {
        const validation = checkAtaAddress(targetAta, mint, owner);
        ataType = validation.type;

        // For wrap=true, must be light-token associated token account
        if (wrap && ataType !== 'ctoken') {
            throw new Error(
                `For wrap=true, targetAta must be light-token associated token account. Got ${ataType} associated token account.`,
            );
        }
    }

    // Check sources for balances (skip frozen -- cannot wrap/decompress frozen accounts)
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

    // Nothing to load (all balances are zero or frozen)
    if (
        splBalance === BigInt(0) &&
        t22Balance === BigInt(0) &&
        coldBalance === BigInt(0)
    ) {
        return [];
    }

    // Get SPL interface info (needed for wrapping or SPL/T22 decompression)
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

    if (wrap) {
        // UNIFIED MODE: Everything goes to light-token associated token account

        // 1. Create light-token associated token account if needed
        if (!ctokenHotSource) {
            instructions.push(
                createAssociatedTokenAccountInterfaceIdempotentInstruction(
                    payer,
                    ctokenAtaAddress,
                    owner,
                    mint,
                    LIGHT_TOKEN_PROGRAM_ID,
                ),
            );
        }

        // 2. Wrap SPL tokens to light-token
        if (splBalance > BigInt(0) && splInterfaceInfo) {
            instructions.push(
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
        }

        // 3. Wrap T22 tokens to light-token
        if (t22Balance > BigInt(0) && splInterfaceInfo) {
            instructions.push(
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
        }

        // 4. Decompress compressed light-tokens to light-token associated token account
        // Note: v3 interface only supports V2 trees
        // Handles >8 accounts via chunking into multiple instructions
        if (coldBalance > BigInt(0) && coldSources.length > 0) {
            const compressedAccounts =
                getCompressedTokenAccountsFromAtaSources(sources);

            if (compressedAccounts.length > 0) {
                const decompressIxs = await createChunkedDecompressInstructions(
                    rpc,
                    payer,
                    compressedAccounts,
                    ctokenAtaAddress,
                    undefined, // No SPL interface for light-token direct
                    decimals,
                );
                instructions.push(...decompressIxs);
            }
        }
    } else {
        // STANDARD MODE: Decompress to target associated token account type
        // Handles >8 accounts via chunking into multiple instructions

        if (coldBalance > BigInt(0) && coldSources.length > 0) {
            const compressedAccounts =
                getCompressedTokenAccountsFromAtaSources(sources);

            if (compressedAccounts.length > 0) {
                if (ataType === 'ctoken') {
                    // Decompress to light-token associated token account (direct)
                    if (!ctokenHotSource) {
                        instructions.push(
                            createAssociatedTokenAccountInterfaceIdempotentInstruction(
                                payer,
                                ctokenAtaAddress,
                                owner,
                                mint,
                                LIGHT_TOKEN_PROGRAM_ID,
                            ),
                        );
                    }
                    const decompressIxs =
                        await createChunkedDecompressInstructions(
                            rpc,
                            payer,
                            compressedAccounts,
                            ctokenAtaAddress,
                            undefined, // No SPL interface for light-token direct
                            decimals,
                        );
                    instructions.push(...decompressIxs);
                } else if (ataType === 'spl' && splInterfaceInfo) {
                    // Decompress to SPL associated token account via interface PDA
                    // Create SPL associated token account if needed
                    if (!splSource) {
                        instructions.push(
                            createAssociatedTokenAccountIdempotentInstruction(
                                payer,
                                splAta,
                                owner,
                                mint,
                                TOKEN_PROGRAM_ID,
                            ),
                        );
                    }
                    const decompressIxs =
                        await createChunkedDecompressInstructions(
                            rpc,
                            payer,
                            compressedAccounts,
                            splAta,
                            splInterfaceInfo,
                            decimals,
                        );
                    instructions.push(...decompressIxs);
                } else if (ataType === 'token2022' && splInterfaceInfo) {
                    // Decompress to T22 associated token account via interface PDA
                    // Create T22 associated token account if needed
                    if (!t22Source) {
                        instructions.push(
                            createAssociatedTokenAccountIdempotentInstruction(
                                payer,
                                t22Ata,
                                owner,
                                mint,
                                TOKEN_2022_PROGRAM_ID,
                            ),
                        );
                    }
                    const decompressIxs =
                        await createChunkedDecompressInstructions(
                            rpc,
                            payer,
                            compressedAccounts,
                            t22Ata,
                            splInterfaceInfo,
                            decimals,
                        );
                    instructions.push(...decompressIxs);
                }
            }
        }
    }

    return instructions;
}

/**
 * Create instruction batches for loading token balances into an associated token account.
 * Handles >8 compressed accounts by returning multiple transaction batches.
 *
 * IMPORTANT: Each batch must be sent as a SEPARATE transaction because
 * multiple decompress instructions in one transaction will have invalid proofs
 * (the Merkle tree root changes after each decompress).
 *
 * @param rpc               RPC connection
 * @param ata               Associated token address (SPL, T22, or light-token)
 * @param owner             Owner public key
 * @param mint              Mint public key
 * @param payer             Fee payer public key (defaults to owner)
 * @param interfaceOptions  Optional interface options
 * @param wrap              Unified mode: wrap SPL/T22 to light-token (default: false)
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

    // Fetch account state (pass wrap so light-token associated token account is validated before RPC)
    let accountInterface: AccountInterface;
    try {
        accountInterface = await _getAtaInterface(
            rpc,
            ata,
            owner,
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

    // Delegate to _buildLoadBatches which handles wrapping, decompression,
    // associated token account creation, and parallel-safe batching.
    const internalBatches = await _buildLoadBatches(
        rpc,
        payer,
        accountInterface,
        interfaceOptions,
        wrap,
        ata,
    );

    // Map InternalLoadBatch[] -> TransactionInstruction[][]
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
 * - Associated token account creation: ~30k CU
 * - Wrap operation: ~50k CU each
 * - Decompress base cost (CPI overhead, hash computation): ~50k CU
 * - Full proof verification (when any input is NOT proveByIndex): ~100k CU
 * - Per compressed account: ~10k (proveByIndex) or ~30k (full proof) CU
 */
/** @internal Exported for use by createTransferInterfaceInstructions. */
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
 * - Batch 0: setup (associated token account creation, wraps) + first decompress chunk
 * - Batch 1..N: idempotent associated token account creation + decompress chunk 1..N
 *
 * Each batch is independent and can be sent in parallel. Idempotent associated token account
 * creation is included in every batch so they can land in any order.
 *
 * @internal
 */
/** @internal Exported for use by createTransferInterfaceInstructions. */
export async function _buildLoadBatches(
    rpc: Rpc,
    payer: PublicKey,
    ata: AccountInterface,
    options: InterfaceOptions | undefined,
    wrap: boolean,
    targetAta: PublicKey,
    targetAmount?: bigint,
): Promise<InternalLoadBatch[]> {
    if (!ata._isAta || !ata._owner || !ata._mint) {
        throw new Error(
            'AccountInterface must be from getAtaInterface (requires _isAta, _owner, _mint)',
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

    // Validate target associated token account type
    let ataType: AtaType = 'ctoken';
    const validation = checkAtaAddress(targetAta, mint, owner);
    ataType = validation.type;
    if (wrap && ataType !== 'ctoken') {
        throw new Error(
            `For wrap=true, targetAta must be light-token associated token account. Got ${ataType} associated token account.`,
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

    // Build setup instructions (associated token account creation + wraps)
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

    // Build idempotent associated token account creation instruction for subsequent batches
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
            // First batch includes all setup (associated token account creation + wraps)
            batchIxs.push(...setupInstructions);
            batchWrapCount = wrapCount;
            batchHasAtaCreation = needsAtaCreation;
        } else {
            // Subsequent batches: include idempotent associated token account creation so
            // batches can land in any order
            batchIxs.push(idempotentAtaIx);
            batchHasAtaCreation = true;
        }

        batchIxs.push(
            createDecompressInterfaceInstruction(
                payer,
                chunk,
                decompressTarget,
                chunkAmount,
                proof,
                decompressSplInfo,
                decimals,
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
 * Load token balances into an associated token account.
 *
 * Behavior depends on `wrap` parameter:
 * - wrap=false (standard): Decompress compressed light-tokens to the target associated token account.
 *   Target can be SPL (via interface PDA), T22 (via interface PDA), or light-token (direct).
 * - wrap=true (unified): Wrap SPL/T22 + decompress all to light-token associated token account.
 *
 * Handles any number of compressed accounts by building per-chunk batches
 * (max 8 inputs per decompress instruction) and sending all batches in
 * parallel. Each batch includes idempotent associated token account creation so landing order
 * does not matter.
 *
 * Idempotent: returns null if nothing to load.
 *
 * @param rpc               RPC connection
 * @param ata               Associated token address (SPL, T22, or light-token)
 * @param owner             Owner of the tokens (signer)
 * @param mint              Mint public key
 * @param payer             Fee payer (signer, defaults to owner)
 * @param confirmOptions    Optional confirm options
 * @param interfaceOptions  Optional interface options
 * @param wrap              Unified mode: wrap SPL/T22 to light-token (default: false)
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

    // Get account interface
    let ataInterface: AccountInterface;
    try {
        ataInterface = await _getAtaInterface(
            rpc,
            ata,
            owner.publicKey,
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

    // Build batched instructions
    const batches = await _buildLoadBatches(
        rpc,
        payer.publicKey,
        ataInterface,
        interfaceOptions,
        wrap,
        ata,
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

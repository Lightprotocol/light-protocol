import {
    Rpc,
    CTOKEN_PROGRAM_ID,
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
 * Create a single decompress instruction for compressed accounts.
 * Limited to MAX_INPUT_ACCOUNTS (8) accounts per call.
 *
 * @param rpc                RPC connection
 * @param payer              Fee payer
 * @param compressedAccounts Compressed accounts to decompress (max 8)
 * @param destinationAta     Destination ATA address
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
                `Use createLoadAtaInstructionBatches for >8 accounts.`,
        );
    }

    assertV2Only(compressedAccounts);

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
 * @param destinationAta     Destination ATA address
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

/**
 * Create instructions to load token balances into an ATA.
 *
 * Behavior depends on `wrap` parameter:
 * - wrap=false (standard): Decompress compressed tokens to the target ATA.
 *   ATA can be SPL (via pool), T22 (via pool), or c-token (direct).
 * - wrap=true (unified): Wrap SPL/T22 + decompress all to c-token ATA.
 *   ATA must be a c-token ATA.
 *
 * @param rpc     RPC connection
 * @param ata     Associated token address (SPL, T22, or c-token)
 * @param owner   Owner public key
 * @param mint    Mint public key
 * @param payer   Fee payer (defaults to owner)
 * @param options Optional load options
 * @param wrap    Unified mode: wrap SPL/T22 to c-token (default: false)
 * @returns       Array of instructions (empty if nothing to load)
 */
export async function createLoadAtaInstructions(
    rpc: Rpc,
    ata: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    payer?: PublicKey,
    options?: InterfaceOptions,
    wrap = false,
): Promise<TransactionInstruction[]> {
    assertBetaEnabled();

    payer ??= owner;

    // Validation happens inside getAtaInterface via checkAtaAddress helper:
    // - Always validates ata matches mint+owner derivation
    // - For wrap=true, additionally requires c-token ATA
    try {
        const ataInterface = await _getAtaInterface(
            rpc,
            ata,
            owner,
            mint,
            undefined,
            undefined,
            wrap,
        );
        return createLoadAtaInstructionsFromInterface(
            rpc,
            payer,
            ataInterface,
            options,
            wrap,
            ata,
        );
    } catch (error) {
        // If account doesn't exist, there's nothing to load
        if (error instanceof TokenAccountNotFoundError) {
            return [];
        }
        throw error;
    }
}

// Re-export AtaType for backwards compatibility
export { AtaType } from '../ata-utils';

/**
 * Create instructions to load an ATA from its AccountInterface.
 *
 * Behavior depends on `wrap` parameter:
 * - wrap=false (standard): Decompress compressed tokens to the target ATA type
 *   (SPL ATA via pool, T22 ATA via pool, or c-token ATA direct)
 * - wrap=true (unified): Wrap SPL/T22 + decompress all to c-token ATA
 *
 * @param rpc         RPC connection
 * @param payer       Fee payer
 * @param ata         AccountInterface from getAtaInterface (must have _isAta, _owner, _mint)
 * @param options     Optional load options
 * @param wrap        Unified mode: wrap SPL/T22 to c-token (default: false)
 * @param targetAta   Target ATA address (used for type detection in standard mode)
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

    // v3 interface only supports V2 trees - check cold sources early
    const compressedAccountsToCheck =
        getCompressedTokenAccountsFromAtaSources(sources);
    if (compressedAccountsToCheck.length > 0) {
        assertV2Only(compressedAccountsToCheck);
    }

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

    // Validate and detect target ATA type
    // If called via createLoadAtaInstructions, validation already happened in getAtaInterface.
    // If called directly, this validates the targetAta is correct.
    let ataType: AtaType = 'ctoken';
    if (targetAta) {
        const validation = checkAtaAddress(targetAta, mint, owner);
        ataType = validation.type;

        // For wrap=true, must be c-token ATA
        if (wrap && ataType !== 'ctoken') {
            throw new Error(
                `For wrap=true, targetAta must be c-token ATA. Got ${ataType} ATA.`,
            );
        }
    }

    // Check sources for balances
    // Note: There can be multiple cold sources (one per compressed account)
    const splSource = sources.find(s => s.type === 'spl');
    const t22Source = sources.find(s => s.type === 'token2022');
    const ctokenHotSource = sources.find(s => s.type === 'ctoken-hot');
    const ctokenColdSources = sources.filter(s => s.type === 'ctoken-cold');

    const splBalance = splSource?.amount ?? BigInt(0);
    const t22Balance = t22Source?.amount ?? BigInt(0);
    // Sum ALL cold balances, not just the first
    const coldBalance = ctokenColdSources.reduce(
        (sum, s) => sum + s.amount,
        BigInt(0),
    );

    // Nothing to load
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
        } catch {
            // No SPL interface exists
        }
    }

    if (wrap) {
        // UNIFIED MODE: Everything goes to c-token ATA

        // 1. Create c-token ATA if needed
        if (!ctokenHotSource) {
            instructions.push(
                createAssociatedTokenAccountInterfaceIdempotentInstruction(
                    payer,
                    ctokenAtaAddress,
                    owner,
                    mint,
                    CTOKEN_PROGRAM_ID,
                ),
            );
        }

        // 2. Wrap SPL tokens to c-token
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

        // 3. Wrap T22 tokens to c-token
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

        // 4. Decompress compressed tokens to c-token ATA
        // Note: v3 interface only supports V2 trees
        // Handles >8 accounts via chunking into multiple instructions
        if (coldBalance > BigInt(0) && ctokenColdSources.length > 0) {
            const compressedAccounts =
                getCompressedTokenAccountsFromAtaSources(sources);

            if (compressedAccounts.length > 0) {
                const decompressIxs = await createChunkedDecompressInstructions(
                    rpc,
                    payer,
                    compressedAccounts,
                    ctokenAtaAddress,
                    undefined, // No SPL interface for c-token direct
                    decimals,
                );
                instructions.push(...decompressIxs);
            }
        }
    } else {
        // STANDARD MODE: Decompress to target ATA type
        // Handles >8 accounts via chunking into multiple instructions

        if (coldBalance > BigInt(0) && ctokenColdSources.length > 0) {
            const compressedAccounts =
                getCompressedTokenAccountsFromAtaSources(sources);

            if (compressedAccounts.length > 0) {
                if (ataType === 'ctoken') {
                    // Decompress to c-token ATA (direct)
                    if (!ctokenHotSource) {
                        instructions.push(
                            createAssociatedTokenAccountInterfaceIdempotentInstruction(
                                payer,
                                ctokenAtaAddress,
                                owner,
                                mint,
                                CTOKEN_PROGRAM_ID,
                            ),
                        );
                    }
                    const decompressIxs =
                        await createChunkedDecompressInstructions(
                            rpc,
                            payer,
                            compressedAccounts,
                            ctokenAtaAddress,
                            undefined, // No SPL interface for c-token direct
                            decimals,
                        );
                    instructions.push(...decompressIxs);
                } else if (ataType === 'spl' && splInterfaceInfo) {
                    // Decompress to SPL ATA via token pool
                    // Create SPL ATA if needed
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
                    // Decompress to T22 ATA via token pool
                    // Create T22 ATA if needed
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
 * Result type for createLoadAtaInstructionBatches
 */
export interface LoadAtaInstructionBatches {
    /** Array of instruction batches - each batch is one transaction */
    batches: TransactionInstruction[][];
    /** Total number of compressed accounts being processed */
    totalCompressedAccounts: number;
}

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
 * @returns Instruction batches and metadata
 */
export async function createLoadAtaInstructionBatches(
    rpc: Rpc,
    ata: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    payer?: PublicKey,
    interfaceOptions?: InterfaceOptions,
    wrap = false,
): Promise<LoadAtaInstructionBatches> {
    assertBetaEnabled();
    payer ??= owner;

    // Determine target ATA type
    const { type: ataType } = checkAtaAddress(ata, mint, owner);

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

    // Fetch account state and sources
    const accountInterface = await _getAtaInterface(rpc, ata, owner, mint);
    const sources = accountInterface._sources ?? [];

    // Get cold sources
    const ctokenColdSources = sources.filter(
        s => s.type === TokenAccountSourceType.CTokenCold,
    );

    const coldBalance = ctokenColdSources.reduce(
        (sum, s) => sum + s.amount,
        BigInt(0),
    );

    // If no cold balance, return empty
    if (coldBalance === BigInt(0) || ctokenColdSources.length === 0) {
        return { batches: [], totalCompressedAccounts: 0 };
    }

    // Get decimals
    const mintInfo = await getMint(rpc, mint).catch(() => null);
    const decimals = mintInfo?.decimals ?? 9;

    // Get all compressed accounts
    const compressedAccounts =
        getCompressedTokenAccountsFromAtaSources(sources);
    const totalCompressedAccounts = compressedAccounts.length;

    // Determine target ATA and SPL interface info
    let targetAta: PublicKey;
    let splInterfaceInfo: SplInterfaceInfo | undefined;

    if (wrap) {
        targetAta = ctokenAtaAddress;
        splInterfaceInfo = undefined;
    } else if (ataType === 'ctoken') {
        targetAta = ctokenAtaAddress;
        splInterfaceInfo = undefined;
    } else {
        // For SPL/T22, we need the interface info
        const splInterfaceInfos = await getSplInterfaceInfos(rpc, mint);
        if (ataType === 'spl') {
            targetAta = splAta;
            splInterfaceInfo = splInterfaceInfos.find(info =>
                info.tokenProgram.equals(TOKEN_PROGRAM_ID),
            );
        } else {
            targetAta = t22Ata;
            splInterfaceInfo = splInterfaceInfos.find(info =>
                info.tokenProgram.equals(TOKEN_2022_PROGRAM_ID),
            );
        }
    }

    // Split into chunks
    const chunks = chunkArray(compressedAccounts, MAX_INPUT_ACCOUNTS);
    const batches: TransactionInstruction[][] = [];

    // Check if we need to create the ATA
    const ctokenHotSource = sources.find(
        s => s.type === TokenAccountSourceType.CTokenHot,
    );
    const splSource = sources.find(s => s.type === TokenAccountSourceType.Spl);
    const t22Source = sources.find(
        s => s.type === TokenAccountSourceType.Token2022,
    );

    for (let i = 0; i < chunks.length; i++) {
        const chunk = chunks[i];
        const batchInstructions: TransactionInstruction[] = [];

        // First batch includes ATA creation if needed
        if (i === 0) {
            if (wrap || ataType === 'ctoken') {
                if (!ctokenHotSource) {
                    batchInstructions.push(
                        createAssociatedTokenAccountInterfaceIdempotentInstruction(
                            payer,
                            ctokenAtaAddress,
                            owner,
                            mint,
                            CTOKEN_PROGRAM_ID,
                        ),
                    );
                }
            } else if (ataType === 'spl' && !splSource) {
                batchInstructions.push(
                    createAssociatedTokenAccountIdempotentInstruction(
                        payer,
                        splAta,
                        owner,
                        mint,
                        TOKEN_PROGRAM_ID,
                    ),
                );
            } else if (ataType === 'token2022' && !t22Source) {
                batchInstructions.push(
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

        // Add decompress instruction for this chunk
        const decompressIx = await createDecompressInstructionForAccounts(
            rpc,
            payer,
            chunk,
            targetAta,
            splInterfaceInfo,
            decimals,
        );
        batchInstructions.push(decompressIx);

        batches.push(batchInstructions);
    }

    return { batches, totalCompressedAccounts };
}

/**
 * Load token balances into an ATA.
 *
 * Behavior depends on `wrap` parameter:
 * - wrap=false (standard): Decompress compressed tokens to the target ATA.
 *   ATA can be SPL (via pool), T22 (via pool), or c-token (direct).
 * - wrap=true (unified): Wrap SPL/T22 + decompress all to c-token ATA.
 *
 * Handles >8 compressed accounts by sending multiple transactions sequentially.
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

    const ixs = await createLoadAtaInstructions(
        rpc,
        ata,
        owner.publicKey,
        mint,
        payer.publicKey,
        interfaceOptions,
        wrap,
    );

    if (ixs.length === 0) {
        return null;
    }

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);

    // Scale CU based on number of decompress instructions
    const decompressIxCount = ixs.filter(
        ix => ix.programId.equals(CTOKEN_PROGRAM_ID) && ix.data.length > 50,
    ).length;
    const computeUnits = Math.min(
        1_400_000,
        500_000 + decompressIxCount * 100_000,
    );

    const tx = buildAndSignTx(
        [
            ComputeBudgetProgram.setComputeUnitLimit({ units: computeUnits }),
            ...ixs,
        ],
        payer,
        blockhash,
        additionalSigners,
    );

    return sendAndConfirmTx(rpc, tx, confirmOptions);
}

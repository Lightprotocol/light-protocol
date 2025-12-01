import {
    Rpc,
    MerkleContext,
    ValidityProof,
    packDecompressAccountsIdempotent,
    CTOKEN_PROGRAM_ID,
    buildAndSignTx,
    sendAndConfirmTx,
    dedupeSigner,
} from '@lightprotocol/stateless.js';
import {
    PublicKey,
    AccountMeta,
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
} from '@solana/spl-token';
import {
    AccountInterface,
    getAtaInterface,
} from '../mint/get-account-interface';
import { getAtaAddressInterface } from '../mint/actions/create-ata-interface';
import { createAssociatedTokenAccountInterfaceIdempotentInstruction } from '../mint/instructions/create-associated-ctoken';
import { createWrapInstruction } from '../mint/instructions/wrap';
import { createDecompress2Instruction } from '../mint/instructions/decompress2';
import {
    getTokenPoolInfos,
    TokenPoolInfo,
} from '../utils/get-token-pool-infos';
import { getAtaProgramId } from '../utils';
import { InterfaceOptions } from '../mint';

/**
 * Account info interface for compressible accounts.
 * Matches return structure of getAccountInterface/getAtaInterface.
 *
 * Integrating programs provide their own fetch/parse - this is just the data shape.
 */
export interface ParsedAccountInfoInterface<T = unknown> {
    /** Parsed account data (program-specific) */
    parsed: T;
    /** Load context - present if account is compressed (cold), undefined if hot */
    loadContext?: MerkleContext;
}

/**
 * Input for buildLoadParams.
 * Supports both program PDAs and CToken vaults.
 *
 * The integrating program is responsible for fetching and parsing their accounts.
 * This helper just packs them for the decompressAccountsIdempotent instruction.
 */
export interface CompressibleAccountInput<T = unknown> {
    /** Account address */
    address: PublicKey;
    /**
     * Account type key for packing:
     * - For PDAs: program-specific type name (e.g., "poolState", "observationState")
     * - For CToken vaults: "cTokenData"
     */
    accountType: string;
    /**
     * Token variant - required when accountType is "cTokenData".
     * Examples: "lpVault", "token0Vault", "token1Vault"
     */
    tokenVariant?: string;
    /** Parsed account info (from program-specific fetch) */
    info: ParsedAccountInfoInterface<T>;
}

/**
 * Packed compressed account for decompressAccountsIdempotent instruction
 */
export interface PackedCompressedAccount {
    [key: string]: unknown;
    merkleContext: {
        merkleTreePubkeyIndex: number;
        queuePubkeyIndex: number;
    };
}

/**
 * Result from building load params
 */
export interface CompressibleLoadParams {
    /** Validity proof wrapped in option (null if all proveByIndex) */
    proofOption: { 0: ValidityProof | null };
    /** Packed compressed accounts data for instruction */
    compressedAccounts: PackedCompressedAccount[];
    /** Offset to system accounts in remainingAccounts */
    systemAccountsOffset: number;
    /** Account metas for remaining accounts */
    remainingAccounts: AccountMeta[];
}

/**
 * Result from buildLoadParams
 */
export interface LoadResult {
    /** Params for decompressAccountsIdempotent (null if no program accounts need decompressing) */
    decompressParams: CompressibleLoadParams | null;
    /** Instructions to load ATAs (create ATA, wrap SPL/T22, decompress2) */
    ataInstructions: TransactionInstruction[];
}

// ============================================
// Shared helper: Build load instructions from AccountInterface
// ============================================

/**
 * Build instructions to load an ATA from its AccountInterface.
 *
 * This creates instructions to:
 * 1. Create CToken ATA if needed (idempotent)
 * 2. Wrap SPL tokens to CToken ATA (if SPL balance > 0)
 * 3. Wrap T22 tokens to CToken ATA (if T22 balance > 0)
 * 4. Decompress2 compressed tokens to CToken ATA (if cold balance > 0)
 *
 * @param rpc     RPC connection
 * @param payer   Fee payer
 * @param ata     AccountInterface from getAtaInterface (must have _isAta, _owner, _mint)
 * @param options Optional load options
 * @returns       Array of instructions (empty if nothing to load)
 */
export async function buildAtaLoadInstructions(
    rpc: Rpc,
    payer: PublicKey,
    ata: AccountInterface,
    options?: InterfaceOptions,
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

    // Derive addresses
    const ctokenAta = getAtaAddressInterface(mint, owner);
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

    // Check sources for balances
    const splSource = sources.find(s => s.type === 'spl');
    const t22Source = sources.find(s => s.type === 'token2022');
    const ctokenHotSource = sources.find(s => s.type === 'ctoken-hot');
    const ctokenColdSource = sources.find(s => s.type === 'ctoken-cold');

    const splBalance = splSource?.amount ?? BigInt(0);
    const t22Balance = t22Source?.amount ?? BigInt(0);
    const coldBalance = ctokenColdSource?.amount ?? BigInt(0);

    // Nothing to load
    if (
        splBalance === BigInt(0) &&
        t22Balance === BigInt(0) &&
        coldBalance === BigInt(0)
    ) {
        return [];
    }

    // 1. Create CToken ATA if needed (idempotent)
    if (!ctokenHotSource) {
        instructions.push(
            createAssociatedTokenAccountInterfaceIdempotentInstruction(
                payer,
                ctokenAta,
                owner,
                mint,
                CTOKEN_PROGRAM_ID,
            ),
        );
    }

    // Get token pool info for wrap operations
    const tokenPoolInfos =
        options?.tokenPoolInfos ?? (await getTokenPoolInfos(rpc, mint));
    const tokenPoolInfo = tokenPoolInfos.find(
        (info: TokenPoolInfo) => info.isInitialized,
    );

    // 2. Wrap SPL tokens
    if (splBalance > BigInt(0) && tokenPoolInfo) {
        instructions.push(
            createWrapInstruction(
                splAta,
                ctokenAta,
                owner,
                mint,
                splBalance,
                tokenPoolInfo,
                payer,
            ),
        );
    }

    // 3. Wrap T22 tokens
    if (t22Balance > BigInt(0) && tokenPoolInfo) {
        instructions.push(
            createWrapInstruction(
                t22Ata,
                ctokenAta,
                owner,
                mint,
                t22Balance,
                tokenPoolInfo,
                payer,
            ),
        );
    }

    // 4. Decompress2 compressed tokens
    if (coldBalance > BigInt(0) && ctokenColdSource) {
        // Need to fetch compressed accounts for decompress2 instruction
        const compressedResult = await rpc.getCompressedTokenAccountsByOwner(
            owner,
            { mint },
        );
        const compressedAccounts = compressedResult.items;

        if (compressedAccounts.length > 0) {
            const proof = await rpc.getValidityProofV0(
                compressedAccounts.map(acc => ({
                    hash: acc.compressedAccount.hash,
                    tree: acc.compressedAccount.treeInfo.tree,
                    queue: acc.compressedAccount.treeInfo.queue,
                })),
            );

            instructions.push(
                createDecompress2Instruction(
                    payer,
                    compressedAccounts,
                    ctokenAta,
                    coldBalance,
                    proof.compressedProof,
                    proof.rootIndices,
                ),
            );
        }
    }

    return instructions;
}

/**
 * Alias for buildAtaLoadInstructions.
 * Use when you have a pre-fetched AccountInterface.
 */
export const loadAtaInstructionsFromInterface = buildAtaLoadInstructions;

/**
 * Build instructions to load an ATA.
 *
 * Fetches the AccountInterface internally, then builds instructions to:
 * 1. Create CToken ATA if needed (idempotent)
 * 2. Wrap SPL tokens to CToken ATA (if SPL balance > 0)
 * 3. Wrap T22 tokens to CToken ATA (if T22 balance > 0)
 * 4. Decompress2 compressed tokens to CToken ATA (if cold balance > 0)
 *
 * @param rpc     RPC connection
 * @param payer   Fee payer
 * @param ata     CToken ATA address (from getAtaAddressInterface)
 * @param owner   ATA owner
 * @param mint    Token mint
 * @param options Optional load options
 * @returns       Array of instructions (empty if nothing to load)
 *
 * @example
 * ```typescript
 * const ata = getAtaAddressInterface(mint, sender);
 * const instructions = await loadAtaInstructions(rpc, payer, ata, sender, mint);
 * ```
 */
export async function loadAtaInstructions(
    rpc: Rpc,
    payer: PublicKey,
    ata: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    options?: InterfaceOptions,
): Promise<TransactionInstruction[]> {
    const ataInterface = await getAtaInterface(rpc, owner, mint);
    return buildAtaLoadInstructions(rpc, payer, ataInterface, options);
}

/**
 * Load ALL token balances into a single CToken ATA (ATA-only, full execute).
 *
 * This loads:
 * 1. SPL ATA balance → wrapped to CToken ATA
 * 2. Token-2022 ATA balance → wrapped to CToken ATA
 * 3. All compressed tokens → decompressed to CToken ATA
 *
 * Idempotent: returns null if nothing to load.
 *
 * @param rpc             RPC connection
 * @param payer           Fee payer (signer)
 * @param ata             CToken ATA address (from getAtaAddressInterface)
 * @param owner           Owner of the tokens (signer)
 * @param mint            Mint address
 * @param confirmOptions  Optional confirm options
 * @param options         Optional interface options
 * @returns Transaction signature, or null if nothing to load
 *
 * @example
 * ```typescript
 * const ata = getAtaAddressInterface(mint, sender);
 * const signature = await loadAta(rpc, payer, ata, sender, mint);
 * ```
 */
export async function loadAta(
    rpc: Rpc,
    payer: Signer,
    ata: PublicKey,
    owner: Signer,
    mint: PublicKey,
    confirmOptions?: ConfirmOptions,
    options?: InterfaceOptions,
): Promise<TransactionSignature | null> {
    const ixs = await loadAtaInstructions(
        rpc,
        payer.publicKey,
        ata,
        owner.publicKey,
        mint,
        options,
    );

    if (ixs.length === 0) {
        return null;
    }

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);

    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }), ...ixs],
        payer,
        blockhash,
        additionalSigners,
    );

    return sendAndConfirmTx(rpc, tx, confirmOptions);
}

// ============================================
// Main function: buildLoadParams
// ============================================

/**
 * Build params for loading program accounts and ATAs.
 *
 * Returns:
 * - decompressParams: for custom program's decompressAccountsIdempotent instruction
 * - ataInstructions: for loading user ATAs (create ATA, wrap SPL/T22, decompress2)
 *
 * @param rpc              RPC connection
 * @param payer            Fee payer (needed for ATA instructions)
 * @param programId        Program ID for decompressAccountsIdempotent
 * @param programAccounts  PDAs and vaults (caller pre-fetches)
 * @param atas             User ATAs (fetched via getAtaInterface)
 * @param options          Optional load options
 * @returns                LoadResult with decompressParams and ataInstructions
 *
 * @example
 * ```typescript
 * const poolInfo = await myProgram.fetchPoolState(rpc, poolAddress);
 * const vault0Info = await getAtaInterface(rpc, poolAddress, token0Mint, undefined, CTOKEN_PROGRAM_ID);
 * const userAta = await getAtaInterface(rpc, userWallet, tokenMint);
 *
 * const result = await buildLoadParams(
 *     rpc,
 *     payer.publicKey,
 *     programId,
 *     [
 *         { address: poolAddress, accountType: 'poolState', info: poolInfo },
 *         { address: vault0, accountType: 'cTokenData', tokenVariant: 'token0Vault', info: vault0Info },
 *     ],
 *     [userAta],
 * );
 *
 * // Build transaction with both program decompress and ATA load
 * const instructions = [...result.ataInstructions];
 * if (result.decompressParams) {
 *     instructions.push(await program.methods
 *         .decompressAccountsIdempotent(
 *             result.decompressParams.proofOption,
 *             result.decompressParams.compressedAccounts,
 *             result.decompressParams.systemAccountsOffset,
 *         )
 *         .remainingAccounts(result.decompressParams.remainingAccounts)
 *         .instruction());
 * }
 * ```
 */
export async function buildLoadParams(
    rpc: Rpc,
    payer: PublicKey,
    programId: PublicKey,
    programAccounts: CompressibleAccountInput[] = [],
    atas: AccountInterface[] = [],
    options?: InterfaceOptions,
): Promise<LoadResult> {
    // ============================================
    // 1. Build decompressParams for program accounts
    // ============================================
    let decompressParams: CompressibleLoadParams | null = null;

    const compressedProgramAccounts = programAccounts.filter(
        acc => acc.info.loadContext !== undefined,
    );

    if (compressedProgramAccounts.length > 0) {
    // Build proof inputs
        const proofInputs = compressedProgramAccounts.map(acc => ({
        hash: acc.info.loadContext!.hash,
        tree: acc.info.loadContext!.treeInfo.tree,
        queue: acc.info.loadContext!.treeInfo.queue,
    }));

        // Get validity proof
    const proofResult = await rpc.getValidityProofV0(proofInputs, []);

    // Build accounts data for packing
        const accountsData = compressedProgramAccounts.map(acc => {
        if (acc.accountType === 'cTokenData') {
            if (!acc.tokenVariant) {
                throw new Error(
                    'tokenVariant is required when accountType is "cTokenData"',
                );
            }
            return {
                key: 'cTokenData',
                data: {
                    variant: { [acc.tokenVariant]: {} },
                    tokenData: acc.info.parsed,
                },
                treeInfo: acc.info.loadContext!.treeInfo,
            };
            }
            return {
                key: acc.accountType,
                data: acc.info.parsed,
                treeInfo: acc.info.loadContext!.treeInfo,
            };
    });

        const addresses = compressedProgramAccounts.map(acc => acc.address);
        const treeInfos = compressedProgramAccounts.map(
        acc => acc.info.loadContext!.treeInfo,
    );

    const packed = await packDecompressAccountsIdempotent(
        programId,
        {
            compressedProof: proofResult.compressedProof,
            treeInfos,
        },
        accountsData,
        addresses,
    );

        decompressParams = {
        proofOption: packed.proofOption,
        compressedAccounts:
            packed.compressedAccounts as PackedCompressedAccount[],
        systemAccountsOffset: packed.systemAccountsOffset,
        remainingAccounts: packed.remainingAccounts,
        };
    }

    // ============================================
    // 2. Build ATA load instructions
    // ============================================
    const ataInstructions: TransactionInstruction[] = [];

    for (const ata of atas) {
        const ixs = await buildAtaLoadInstructions(rpc, payer, ata, options);
        ataInstructions.push(...ixs);
    }

    return {
        decompressParams,
        ataInstructions,
    };
}

/**
 * Calculate compute units for compressible load operation
 */
export function calculateCompressibleLoadComputeUnits(
    compressedAccountCount: number,
    hasValidityProof: boolean,
): number {
    let cu = 50_000; // Base

    if (hasValidityProof) {
        cu += 100_000; // Proof verification
    }

    // Per compressed account
    cu += compressedAccountCount * 30_000;

    return cu;
}

// Re-export for backward compatibility
export { buildDecompressParams } from './helpers';
export type { AccountInput, DecompressInstructionParams } from './helpers';

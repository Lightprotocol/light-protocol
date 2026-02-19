import {
    Rpc,
    MerkleContext,
    ValidityProof,
    packDecompressAccountsIdempotent,
} from '@lightprotocol/stateless.js';
import {
    PublicKey,
    AccountMeta,
    TransactionInstruction,
} from '@solana/web3.js';
import { AccountInterface } from '../get-account-interface';
import { _buildLoadBatches } from '../actions/load-ata';
import { getAssociatedTokenAddressInterface } from '../get-associated-token-address-interface';
import { InterfaceOptions } from '../actions/transfer-interface';

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
 * Input for createLoadAccountsParams.
 * Supports both program PDAs and c-token vaults.
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
     * - For c-token vaults: "cTokenData"
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
 * Result from createLoadAccountsParams
 */
export interface LoadResult {
    /** Params for decompressAccountsIdempotent (null if no program accounts need decompressing) */
    decompressParams: CompressibleLoadParams | null;
    /**
     * Instruction batches to load ATAs (create ATA, wrap SPL/T22, decompress).
     * Each inner array is one transaction. When >8 compressed inputs exist for
     * a single ATA, multiple batches are returned and must be sent as separate
     * transactions. Send all batches in parallel, then proceed with the program
     * instruction. Most use cases produce a single batch.
     */
    ataInstructions: TransactionInstruction[][];
}

/**
 * Create params for loading program accounts and ATAs.
 *
 * Returns:
 * - decompressParams: for a caller program's standardized
 *   decompressAccountsIdempotent instruction
 * - ataInstructions: for loading user ATAs
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
 * const vault0Ata = getAssociatedTokenAddressInterface(token0Mint, poolAddress);
 * const vault0Info = await getAtaInterface(rpc, vault0Ata, poolAddress, token0Mint, undefined, LIGHT_TOKEN_PROGRAM_ID);
 * const userAta = getAssociatedTokenAddressInterface(tokenMint, userWallet);
 * const userAtaInfo = await getAtaInterface(rpc, userAta, userWallet, tokenMint);
 *
 * const result = await createLoadAccountsParams(
 *     rpc,
 *     payer.publicKey,
 *     programId,
 *     [
 *         { address: poolAddress, accountType: 'poolState', info: poolInfo },
 *         { address: vault0, accountType: 'cTokenData', tokenVariant: 'token0Vault', info: vault0Info },
 *     ],
 *     [userAtaInfo],
 * );
 *
 * // Send ATA load batches first (parallel when >1 batch), then the program instruction
 * await Promise.all(result.ataInstructions.map(async ixs => {
 *     const { blockhash } = await rpc.getLatestBlockhash();
 *     const tx = buildAndSignTx(ixs, payer, blockhash, [owner]);
 *     return sendAndConfirmTx(rpc, tx);
 * }));
 *
 * // Build and send the program instruction
 * const programIxs: TransactionInstruction[] = [];
 * if (result.decompressParams) {
 *     programIxs.push(await program.methods
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
export async function createLoadAccountsParams(
    rpc: Rpc,
    payer: PublicKey,
    programId: PublicKey,
    programAccounts: CompressibleAccountInput[] = [],
    atas: AccountInterface[] = [],
    options?: InterfaceOptions,
): Promise<LoadResult> {
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

    const ataInstructions: TransactionInstruction[][] = [];

    for (const ata of atas) {
        if (!ata._isAta || !ata._owner || !ata._mint) {
            throw new Error(
                'Each ATA must be from getAtaInterface (requires _isAta, _owner, _mint)',
            );
        }
        const targetAta = getAssociatedTokenAddressInterface(ata._mint, ata._owner);
        const batches = await _buildLoadBatches(rpc, payer, ata, options, false, targetAta);
        for (const batch of batches) {
            ataInstructions.push(batch.instructions);
        }
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

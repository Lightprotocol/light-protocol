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
    CTOKEN_PROGRAM_ID,
    dedupeSigner,
    ParsedTokenAccount,
} from '@lightprotocol/stateless.js';
import {
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    getAssociatedTokenAddressSync,
} from '@solana/spl-token';
import BN from 'bn.js';
import { getAtaProgramId } from '../../utils';
import {
    createTransferInterfaceInstruction,
    createCTokenTransferInstruction,
} from '../instructions/transfer-interface';
import { createAssociatedTokenAccountInterfaceIdempotentInstruction } from '../instructions/create-associated-ctoken';
import { getAtaAddressInterface } from './create-ata-interface';
import {
    getTokenPoolInfos,
    TokenPoolInfo,
} from '../../utils/get-token-pool-infos';
import { createWrapInstruction } from '../instructions/wrap';
import { createDecompress2Instruction } from '../instructions/decompress2';

/**
 * Options for interface operations (load, transfer)
 */
export interface InterfaceOptions {
    /** Token pool infos (fetched if not provided) */
    tokenPoolInfos?: TokenPoolInfo[];
}

/**
 * Calculate compute units needed for the operation
 */
function calculateComputeUnits(
    compressedAccounts: ParsedTokenAccount[],
    hasValidityProof: boolean,
    splWrapCount: number,
): number {
    // Base CU for hot CToken transfer
    let cu = 5_000;

    // Compressed token decompression
    if (compressedAccounts.length > 0) {
        if (hasValidityProof) {
            cu += 100_000; // Validity proof verification
        }
        // Per compressed account
        for (const acc of compressedAccounts) {
            const proveByIndex = acc.compressedAccount.proveByIndex ?? false;
            cu += proveByIndex ? 10_000 : 30_000;
        }
    }

    // SPL/T22 wrap operations
    cu += splWrapCount * 5_000;

    return cu;
}

/**
 * Build instructions to load ALL token balances into a single CToken ATA.
 *
 * This loads:
 * 1. SPL ATA balance (if exists) → wrapped to CToken ATA
 * 2. Token-2022 ATA balance (if exists) → wrapped to CToken ATA
 * 3. All compressed token accounts → decompressed to CToken ATA
 *
 * Idempotent: returns empty instructions if nothing to load.
 *
 * @param rpc       RPC connection
 * @param payer     Fee payer public key
 * @param owner     Owner of the tokens
 * @param mint      Mint address
 * @param options   Optional interface options
 * @returns Load instructions (empty if nothing to load)
 */
export async function loadInstructions(
    rpc: Rpc,
    payer: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    options?: InterfaceOptions,
): Promise<TransactionInstruction[]> {
    const instructions: TransactionInstruction[] = [];
    const { tokenPoolInfos: providedTokenPoolInfos } = options ?? {};

    // Get CToken ATA
    const ctokenAta = getAtaAddressInterface(mint, owner);

    // Derive ATAs for all token programs
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

    // Fetch all accounts in parallel
    const [ctokenAtaInfo, splAtaInfo, t22AtaInfo, compressedResult] =
        await Promise.all([
            rpc.getAccountInfo(ctokenAta),
            rpc.getAccountInfo(splAta),
            rpc.getAccountInfo(t22Ata),
            rpc.getCompressedTokenAccountsByOwner(owner, { mint }),
        ]);

    const compressedAccounts = compressedResult.items;

    // Parse balances
    const splBalance =
        splAtaInfo && splAtaInfo.data.length >= 72
            ? splAtaInfo.data.readBigUInt64LE(64)
            : BigInt(0);
    const t22Balance =
        t22AtaInfo && t22AtaInfo.data.length >= 72
            ? t22AtaInfo.data.readBigUInt64LE(64)
            : BigInt(0);
    const compressedBalance = compressedAccounts.reduce(
        (sum, acc) => sum + BigInt(acc.parsed.amount.toString()),
        BigInt(0),
    );

    // Nothing to load - idempotent
    if (
        splBalance === BigInt(0) &&
        t22Balance === BigInt(0) &&
        compressedBalance === BigInt(0)
    ) {
        return [];
    }

    // Create CToken ATA if needed (idempotent)
    if (!ctokenAtaInfo) {
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

    // Get token pool infos (needed for wrap and decompress)
    const tokenPoolInfos =
        providedTokenPoolInfos ?? (await getTokenPoolInfos(rpc, mint));
    const tokenPoolInfo = tokenPoolInfos.find(info => info.isInitialized);

    // 1. Wrap SPL tokens if balance exists
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

    // 2. Wrap T22 tokens if balance exists
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

    // 3. Decompress ALL compressed tokens if they exist (using Transfer2-based decompress2)
    if (compressedBalance > BigInt(0) && compressedAccounts.length > 0) {
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
                compressedBalance,
                proof.compressedProof,
                proof.rootIndices,
            ),
        );
    }

    return instructions;
}

/**
 * Load ALL token balances into a single CToken ATA.
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
 * @param owner           Owner of the tokens (signer)
 * @param mint            Mint address
 * @param confirmOptions  Optional confirm options
 * @param options         Optional interface options
 * @returns Transaction signature, or null if nothing to load
 */
export async function load(
    rpc: Rpc,
    payer: Signer,
    owner: Signer,
    mint: PublicKey,
    confirmOptions?: ConfirmOptions,
    options?: InterfaceOptions,
): Promise<TransactionSignature | null> {
    const ixs = await loadInstructions(
        rpc,
        payer.publicKey,
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

/**
 * Transfer tokens using the CToken interface.
 *
 * This action:
 * 1. Validates source matches derived ATA from owner + mint
 * 2. Loads ALL balances to CToken ATA (SPL, T22, compressed)
 * 3. Creates destination ATA if it doesn't exist
 * 4. Executes the hot-to-hot transfer
 *
 * @param rpc             RPC connection
 * @param payer           Fee payer (signer)
 * @param source          Source CToken ATA address
 * @param destination     Destination owner public key
 * @param owner           Source owner (signer)
 * @param mint            Mint address
 * @param amount          Amount to transfer
 * @param programId       Token program ID (default: CTOKEN_PROGRAM_ID)
 * @param confirmOptions  Optional confirm options
 * @param options         Optional interface options
 * @returns Transaction signature
 */
export async function transferInterface(
    rpc: Rpc,
    payer: Signer,
    source: PublicKey,
    destination: PublicKey,
    owner: Signer,
    mint: PublicKey,
    amount: number | bigint | BN,
    programId: PublicKey = CTOKEN_PROGRAM_ID,
    confirmOptions?: ConfirmOptions,
    options?: InterfaceOptions,
): Promise<TransactionSignature> {
    const amountBigInt = BigInt(amount.toString());
    const { tokenPoolInfos: providedTokenPoolInfos } = options ?? {};

    const instructions: TransactionInstruction[] = [];

    // For non-CToken programs, use simple SPL transfer (no load)
    if (!programId.equals(CTOKEN_PROGRAM_ID)) {
        const expectedSource = getAssociatedTokenAddressSync(
            mint,
            owner.publicKey,
            false,
            programId,
            getAtaProgramId(programId),
        );
        if (!source.equals(expectedSource)) {
            throw new Error(
                `Source mismatch. Expected ${expectedSource.toBase58()}, got ${source.toBase58()}`,
            );
        }

        const destinationAta = getAssociatedTokenAddressSync(
            mint,
            destination,
            false,
            programId,
            getAtaProgramId(programId),
        );

        instructions.push(
            createAssociatedTokenAccountInterfaceIdempotentInstruction(
                payer.publicKey,
                destinationAta,
                destination,
                mint,
                programId,
            ),
        );

        instructions.push(
            createTransferInterfaceInstruction(
                source,
                destinationAta,
                owner.publicKey,
                amountBigInt,
                [],
                programId,
            ),
        );

        const { blockhash } = await rpc.getLatestBlockhash();
        const tx = buildAndSignTx(
            [
                ComputeBudgetProgram.setComputeUnitLimit({ units: 10_000 }),
                ...instructions,
            ],
            payer,
            blockhash,
            [owner],
        );
        return sendAndConfirmTx(rpc, tx, confirmOptions);
    }

    // CToken transfer
    const expectedSource = getAtaAddressInterface(mint, owner.publicKey);
    if (!source.equals(expectedSource)) {
        throw new Error(
            `Source mismatch. Expected ${expectedSource.toBase58()}, got ${source.toBase58()}`,
        );
    }

    const ctokenAta = getAtaAddressInterface(mint, owner.publicKey);
    const destinationAta = getAtaAddressInterface(mint, destination);

    // Derive ATAs for all token programs
    const splAta = getAssociatedTokenAddressSync(
        mint,
        owner.publicKey,
        false,
        TOKEN_PROGRAM_ID,
        getAtaProgramId(TOKEN_PROGRAM_ID),
    );
    const t22Ata = getAssociatedTokenAddressSync(
        mint,
        owner.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID,
        getAtaProgramId(TOKEN_2022_PROGRAM_ID),
    );

    // Fetch all accounts in parallel
    const [ctokenAtaInfo, splAtaInfo, t22AtaInfo, compressedResult] =
        await Promise.all([
            rpc.getAccountInfo(ctokenAta),
            rpc.getAccountInfo(splAta),
            rpc.getAccountInfo(t22Ata),
            rpc.getCompressedTokenAccountsByOwner(owner.publicKey, { mint }),
        ]);

    const compressedAccounts = compressedResult.items;

    // Parse balances
    const hotBalance =
        ctokenAtaInfo && ctokenAtaInfo.data.length >= 72
            ? ctokenAtaInfo.data.readBigUInt64LE(64)
            : BigInt(0);
    const splBalance =
        splAtaInfo && splAtaInfo.data.length >= 72
            ? splAtaInfo.data.readBigUInt64LE(64)
            : BigInt(0);
    const t22Balance =
        t22AtaInfo && t22AtaInfo.data.length >= 72
            ? t22AtaInfo.data.readBigUInt64LE(64)
            : BigInt(0);
    const compressedBalance = compressedAccounts.reduce(
        (sum, acc) => sum + BigInt(acc.parsed.amount.toString()),
        BigInt(0),
    );

    const totalBalance =
        hotBalance + splBalance + t22Balance + compressedBalance;

    if (totalBalance < amountBigInt) {
        throw new Error(
            `Insufficient balance. Required: ${amountBigInt}, Available: ${totalBalance}`,
        );
    }

    // Track what we're doing for CU calculation
    let splWrapCount = 0;
    let hasValidityProof = false;
    let compressedToLoad: ParsedTokenAccount[] = [];

    // Create CToken ATA if needed (idempotent)
    if (!ctokenAtaInfo) {
        instructions.push(
            createAssociatedTokenAccountInterfaceIdempotentInstruction(
                payer.publicKey,
                ctokenAta,
                owner.publicKey,
                mint,
                CTOKEN_PROGRAM_ID,
            ),
        );
    }

    // Get token pool infos if we need to load
    const needsLoad =
        splBalance > BigInt(0) ||
        t22Balance > BigInt(0) ||
        compressedBalance > BigInt(0);
    const tokenPoolInfos = needsLoad
        ? (providedTokenPoolInfos ?? (await getTokenPoolInfos(rpc, mint)))
        : [];
    const tokenPoolInfo = tokenPoolInfos.find(info => info.isInitialized);

    // Wrap SPL tokens if balance exists
    if (splBalance > BigInt(0) && tokenPoolInfo) {
        instructions.push(
            createWrapInstruction(
                splAta,
                ctokenAta,
                owner.publicKey,
                mint,
                splBalance,
                tokenPoolInfo,
                payer.publicKey,
            ),
        );
        splWrapCount++;
    }

    // Wrap T22 tokens if balance exists
    if (t22Balance > BigInt(0) && tokenPoolInfo) {
        instructions.push(
            createWrapInstruction(
                t22Ata,
                ctokenAta,
                owner.publicKey,
                mint,
                t22Balance,
                tokenPoolInfo,
                payer.publicKey,
            ),
        );
        splWrapCount++;
    }

    // Decompress compressed tokens if they exist (using Transfer2-based decompress2)
    if (compressedBalance > BigInt(0) && compressedAccounts.length > 0) {
        const proof = await rpc.getValidityProofV0(
            compressedAccounts.map(acc => ({
                hash: acc.compressedAccount.hash,
                tree: acc.compressedAccount.treeInfo.tree,
                queue: acc.compressedAccount.treeInfo.queue,
            })),
        );

        hasValidityProof = proof.compressedProof !== null;
        compressedToLoad = compressedAccounts;

        instructions.push(
            createDecompress2Instruction(
                payer.publicKey,
                compressedAccounts,
                ctokenAta,
                compressedBalance,
                proof.compressedProof,
                proof.rootIndices,
            ),
        );
    }

    // Create destination ATA (idempotent)
    instructions.push(
        createAssociatedTokenAccountInterfaceIdempotentInstruction(
            payer.publicKey,
            destinationAta,
            destination,
            mint,
            CTOKEN_PROGRAM_ID,
        ),
    );

    // Add transfer instruction
    instructions.push(
        createCTokenTransferInstruction(
            source,
            destinationAta,
            owner.publicKey,
            amountBigInt,
            payer.publicKey,
        ),
    );

    // Calculate compute units
    const computeUnits = calculateComputeUnits(
        compressedToLoad,
        hasValidityProof,
        splWrapCount,
    );

    // Build and send
    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);

    const tx = buildAndSignTx(
        [
            ComputeBudgetProgram.setComputeUnitLimit({ units: computeUnits }),
            ...instructions,
        ],
        payer,
        blockhash,
        additionalSigners,
    );

    return sendAndConfirmTx(rpc, tx, confirmOptions);
}

// Re-export old names for backwards compatibility
export type LoadOptions = InterfaceOptions;
export type TransferInterfaceOptions = InterfaceOptions;

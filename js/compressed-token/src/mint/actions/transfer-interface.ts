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
import { getAtaInterface } from '../get-account-interface';
import { buildAtaLoadInstructions } from '../../compressible/unified-load';

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
 * Transfer tokens using the CToken interface.
 * Mirrors SPL Token's transfer() - destination must exist.
 *
 * This action:
 * 1. Validates source matches derived ATA from owner + mint
 * 2. Loads ALL sender balances to CToken ATA (SPL, T22, compressed)
 * 3. Executes hot-to-hot transfer
 *
 * Note: Like SPL Token, this does NOT create the destination ATA.
 * Use getOrCreateAtaInterface() first if destination may not exist.
 *
 * @param rpc             RPC connection
 * @param payer           Fee payer (signer)
 * @param source          Source CToken ATA address
 * @param destination     Destination CToken ATA address (must exist)
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

        instructions.push(
            createTransferInterfaceInstruction(
                source,
                destination,
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

    // Derive ATAs for all token programs (sender only)
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

    // Fetch sender's accounts in parallel
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

    // Create sender's CToken ATA if needed (idempotent)
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

    // Decompress compressed tokens if they exist
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

    // Transfer (destination must already exist - like SPL Token)
    instructions.push(
        createCTokenTransferInstruction(
            source,
            destination,
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

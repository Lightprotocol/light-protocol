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
import { assertV2Only } from '../assert-v2-only';
import {
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    getAssociatedTokenAddressSync,
    getMint,
} from '@solana/spl-token';
import BN from 'bn.js';
import { getAtaProgramId } from '../ata-utils';
import {
    createTransferInterfaceInstruction,
    createCTokenTransferInstruction,
} from '../instructions/transfer-interface';
import { createAssociatedTokenAccountInterfaceIdempotentInstruction } from '../instructions/create-ata-interface';
import { getAssociatedTokenAddressInterface } from '../get-associated-token-address-interface';
import {
    getSplInterfaceInfos,
    SplInterfaceInfo,
} from '../../utils/get-token-pool-infos';
import { createWrapInstruction } from '../instructions/wrap';
import { createDecompressInterfaceInstruction } from '../instructions/create-decompress-interface-instruction';

/**
 * Options for interface operations (load, transfer)
 */
export interface InterfaceOptions {
    /** SPL interface infos (fetched if not provided) */
    splInterfaceInfos?: SplInterfaceInfo[];
}

/**
 * Calculate compute units needed for the operation
 */
function calculateComputeUnits(
    compressedAccounts: ParsedTokenAccount[],
    hasValidityProof: boolean,
    splWrapCount: number,
): number {
    // Base CU for hot c-token transfer
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

    // TODO: dynamic
    // return cu;
    return 200_000;
}

/**
 * Transfer tokens using the c-token interface.
 *
 * Matches SPL Token's transferChecked signature order. Destination must exist.
 *
 * @param rpc             RPC connection
 * @param payer           Fee payer (signer)
 * @param source          Source c-token ATA address
 * @param mint            Mint address
 * @param destination     Destination c-token ATA address (must exist)
 * @param owner           Source owner (signer)
 * @param amount          Amount to transfer
 * @param programId       Token program ID (default: CTOKEN_PROGRAM_ID)
 * @param confirmOptions  Optional confirm options
 * @param options         Optional interface options
 * @param wrap            Include SPL/T22 wrapping (default: false)
 * @returns Transaction signature
 */
export async function transferInterface(
    rpc: Rpc,
    payer: Signer,
    source: PublicKey,
    mint: PublicKey,
    destination: PublicKey,
    owner: Signer,
    amount: number | bigint | BN,
    programId: PublicKey = CTOKEN_PROGRAM_ID,
    confirmOptions?: ConfirmOptions,
    options?: InterfaceOptions,
    wrap = false,
): Promise<TransactionSignature> {
    const amountBigInt = BigInt(amount.toString());
    const { splInterfaceInfos: providedSplInterfaceInfos } = options ?? {};

    const instructions: TransactionInstruction[] = [];

    // For non-c-token programs, use simple SPL transfer (no load)
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

    // c-token transfer
    const expectedSource = getAssociatedTokenAddressInterface(
        mint,
        owner.publicKey,
    );
    if (!source.equals(expectedSource)) {
        throw new Error(
            `Source mismatch. Expected ${expectedSource.toBase58()}, got ${source.toBase58()}`,
        );
    }

    const ctokenAtaAddress = getAssociatedTokenAddressInterface(
        mint,
        owner.publicKey,
    );

    // Derive SPL/T22 ATAs only if wrap is true
    let splAta: PublicKey | undefined;
    let t22Ata: PublicKey | undefined;

    if (wrap) {
        splAta = getAssociatedTokenAddressSync(
            mint,
            owner.publicKey,
            false,
            TOKEN_PROGRAM_ID,
            getAtaProgramId(TOKEN_PROGRAM_ID),
        );
        t22Ata = getAssociatedTokenAddressSync(
            mint,
            owner.publicKey,
            false,
            TOKEN_2022_PROGRAM_ID,
            getAtaProgramId(TOKEN_2022_PROGRAM_ID),
        );
    }

    // Fetch sender's accounts in parallel (conditionally include SPL/T22)
    const fetchPromises: Promise<unknown>[] = [
        rpc.getAccountInfo(ctokenAtaAddress),
        rpc.getCompressedTokenAccountsByOwner(owner.publicKey, { mint }),
    ];
    if (wrap && splAta && t22Ata) {
        fetchPromises.push(rpc.getAccountInfo(splAta));
        fetchPromises.push(rpc.getAccountInfo(t22Ata));
    }

    const results = await Promise.all(fetchPromises);
    const ctokenAtaInfo = results[0] as Awaited<
        ReturnType<typeof rpc.getAccountInfo>
    >;
    const compressedResult = results[1] as Awaited<
        ReturnType<typeof rpc.getCompressedTokenAccountsByOwner>
    >;
    const splAtaInfo = wrap
        ? (results[2] as Awaited<ReturnType<typeof rpc.getAccountInfo>>)
        : null;
    const t22AtaInfo = wrap
        ? (results[3] as Awaited<ReturnType<typeof rpc.getAccountInfo>>)
        : null;

    const compressedAccounts = compressedResult.items;

    // Parse balances
    const hotBalance =
        ctokenAtaInfo && ctokenAtaInfo.data.length >= 72
            ? ctokenAtaInfo.data.readBigUInt64LE(64)
            : BigInt(0);
    const splBalance =
        wrap && splAtaInfo && splAtaInfo.data.length >= 72
            ? splAtaInfo.data.readBigUInt64LE(64)
            : BigInt(0);
    const t22Balance =
        wrap && t22AtaInfo && t22AtaInfo.data.length >= 72
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

    // Create sender's c-token ATA if needed (idempotent)
    if (!ctokenAtaInfo) {
        instructions.push(
            createAssociatedTokenAccountInterfaceIdempotentInstruction(
                payer.publicKey,
                ctokenAtaAddress,
                owner.publicKey,
                mint,
                CTOKEN_PROGRAM_ID,
            ),
        );
    }

    // Get SPL interface infos if we need to load
    const needsLoad =
        splBalance > BigInt(0) ||
        t22Balance > BigInt(0) ||
        compressedBalance > BigInt(0);
    const splInterfaceInfos = needsLoad
        ? (providedSplInterfaceInfos ?? (await getSplInterfaceInfos(rpc, mint)))
        : [];
    const splInterfaceInfo = splInterfaceInfos.find(info => info.isInitialized);

    // Fetch mint decimals if we need to wrap
    let decimals = 0;
    if (
        splInterfaceInfo &&
        (splBalance > BigInt(0) || t22Balance > BigInt(0))
    ) {
        const mintInfo = await getMint(
            rpc,
            mint,
            undefined,
            splInterfaceInfo.tokenProgram,
        );
        decimals = mintInfo.decimals;
    }

    // Wrap SPL tokens if balance exists (only when wrap=true)
    if (wrap && splAta && splBalance > BigInt(0) && splInterfaceInfo) {
        instructions.push(
            createWrapInstruction(
                splAta,
                ctokenAtaAddress,
                owner.publicKey,
                mint,
                splBalance,
                splInterfaceInfo,
                decimals,
                payer.publicKey,
            ),
        );
        splWrapCount++;
    }

    // Wrap T22 tokens if balance exists (only when wrap=true)
    if (wrap && t22Ata && t22Balance > BigInt(0) && splInterfaceInfo) {
        instructions.push(
            createWrapInstruction(
                t22Ata,
                ctokenAtaAddress,
                owner.publicKey,
                mint,
                t22Balance,
                splInterfaceInfo,
                decimals,
                payer.publicKey,
            ),
        );
        splWrapCount++;
    }

    // Decompress compressed tokens if they exist
    // Note: v3 interface only supports V2 trees
    if (compressedBalance > BigInt(0) && compressedAccounts.length > 0) {
        assertV2Only(compressedAccounts);

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
            createDecompressInterfaceInstruction(
                payer.publicKey,
                compressedAccounts,
                ctokenAtaAddress,
                compressedBalance,
                proof,
                undefined,
                decimals,
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

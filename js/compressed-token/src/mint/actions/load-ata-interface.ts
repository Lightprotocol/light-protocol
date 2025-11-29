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
    bn,
    ParsedTokenAccount,
    TreeInfo,
    ValidityProof,
    CompressedProof,
    dedupeSigner,
} from '@lightprotocol/stateless.js';
import {
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    getAssociatedTokenAddressSync,
} from '@solana/spl-token';
import BN from 'bn.js';
import { getAtaProgramId } from '../../utils';
import { createAssociatedTokenAccountInterfaceIdempotentInstruction } from '../instructions/create-associated-ctoken';
import { getAtaAddressInterface } from './create-ata-interface';
import {
    getTokenPoolInfos,
    TokenPoolInfo,
    selectTokenPoolInfosForDecompression,
} from '../../utils/get-token-pool-infos';
import { CompressedTokenProgram } from '../../program';
import { selectMinCompressedTokenAccountsForTransfer } from '../../utils';
import { createWrapInstruction } from '../instructions/wrap';

/**
 * Source of tokens found during load discovery
 */
export interface LoadSource {
    type: 'spl' | 'token2022' | 'ctoken-onchain' | 'compressed';
    address: PublicKey;
    amount: bigint;
}

// Keep old interface type for backwards compatibility export
export interface LoadAtaInterfaceInstructionsParams {
    rpc: Rpc;
    owner: PublicKey;
    mint: PublicKey;
    payer: PublicKey;
    mintProgramId?: PublicKey;
    tokenPoolInfos?: TokenPoolInfo[];
    outputStateTreeInfo?: TreeInfo;
}

/**
 * Result from loadAtaInterfaceInstructions
 */
export interface LoadAtaInterfaceInstructionsResult {
    ctokenAta: PublicKey;
    instructions: TransactionInstruction[];
    sources: LoadSource[];
    totalAmount: bigint;
    requiresProof: boolean;
    compressedAccounts?: ParsedTokenAccount[];
}

// Keep old interface type for backwards compatibility export
export interface LoadAtaInterfaceParams {
    rpc: Rpc;
    owner: Signer;
    mint: PublicKey;
    payer: Signer;
    mintProgramId?: PublicKey;
    tokenPoolInfos?: TokenPoolInfo[];
    outputStateTreeInfo?: TreeInfo;
    confirmOptions?: ConfirmOptions;
}

/**
 * Result from loadAtaInterface action
 */
export interface LoadAtaInterfaceResult {
    ctokenAta: PublicKey;
    transactionSignature: TransactionSignature;
    sources: LoadSource[];
    totalAmount: bigint;
}

/**
 * Load-specific options (optional config object at end of positional args)
 */
export interface LoadAtaOptions {
    mintProgramId?: PublicKey;
    tokenPoolInfos?: TokenPoolInfo[];
    outputStateTreeInfo?: TreeInfo;
}

/**
 * Get the SPL/T22 token program for a given mint
 */
async function getMintTokenProgram(
    rpc: Rpc,
    mint: PublicKey,
): Promise<PublicKey> {
    const mintInfo = await rpc.getAccountInfo(mint);
    if (!mintInfo) {
        throw new Error(`Mint account not found: ${mint.toBase58()}`);
    }

    if (mintInfo.owner.equals(TOKEN_PROGRAM_ID)) {
        return TOKEN_PROGRAM_ID;
    } else if (mintInfo.owner.equals(TOKEN_2022_PROGRAM_ID)) {
        return TOKEN_2022_PROGRAM_ID;
    } else {
        throw new Error(
            `Unknown mint program: ${mintInfo.owner.toBase58()}. Expected SPL Token or Token-2022.`,
        );
    }
}

/**
 * Build instructions to load all token balances into a single CToken ATA.
 *
 * This instruction builder:
 * 1. Creates CToken ATA if it doesn't exist (idempotent)
 * 2. Wraps SPL/T22 tokens to CToken ATA if SPL/T22 ATA has balance
 * 3. Decompresses compressed tokens to CToken ATA if compressed tokens exist
 *
 * @param rpc             RPC connection
 * @param payer           Fee payer public key
 * @param mint            Mint address
 * @param owner           Owner public key
 * @param options         Optional: Load-specific options (mintProgramId, tokenPoolInfos, outputStateTreeInfo)
 * @returns Instructions and metadata about the load operation
 */
export async function loadAtaInterfaceInstructions(
    rpc: Rpc,
    payer: PublicKey,
    mint: PublicKey,
    owner: PublicKey,
    options?: LoadAtaOptions,
): Promise<LoadAtaInterfaceInstructionsResult> {
    const {
        mintProgramId,
        tokenPoolInfos: providedTokenPoolInfos,
        outputStateTreeInfo: providedStateTreeInfo,
    } = options ?? {};

    const instructions: TransactionInstruction[] = [];
    const sources: LoadSource[] = [];
    let totalAmount = BigInt(0);
    let requiresProof = false;
    let compressedAccountsForProof: ParsedTokenAccount[] | undefined;

    // Get mint's token program (skip lookup for CToken mints)
    const mintTokenProgram =
        mintProgramId ?? (await getMintTokenProgram(rpc, mint));
    const isCTokenMint = mintTokenProgram.equals(CTOKEN_PROGRAM_ID);

    // Derive CToken ATA address (defaults to CTOKEN_PROGRAM_ID)
    const ctokenAta = getAtaAddressInterface(mint, owner);

    // For CToken mints, there's no SPL ATA to check
    const splT22Ata = isCTokenMint
        ? null
        : getAssociatedTokenAddressSync(
              mint,
              owner,
              false,
              mintTokenProgram,
              getAtaProgramId(mintTokenProgram),
          );

    // Fetch account states in parallel
    const [ctokenAtaInfo, splT22AtaInfo, compressedTokensResult] =
        await Promise.all([
            rpc.getAccountInfo(ctokenAta),
            splT22Ata ? rpc.getAccountInfo(splT22Ata) : Promise.resolve(null),
            rpc.getCompressedTokenAccountsByOwner(owner, { mint }),
        ]);

    // 1. Create CToken ATA if it doesn't exist
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

    // 2. Wrap SPL/T22 tokens if they exist (skip for CToken mints)
    if (
        !isCTokenMint &&
        splT22Ata &&
        splT22AtaInfo &&
        splT22AtaInfo.data.length >= 72
    ) {
        // Parse token account balance (offset 64-72 for amount in SPL token account layout)
        const balance = splT22AtaInfo.data.readBigUInt64LE(64);

        if (balance > BigInt(0)) {
            // Get token pool infos for wrap operation
            const tokenPoolInfos =
                providedTokenPoolInfos ?? (await getTokenPoolInfos(rpc, mint));
            const tokenPoolInfo = tokenPoolInfos.find(
                info => info.isInitialized,
            );

            if (!tokenPoolInfo) {
                throw new Error(
                    `No initialized token pool found for mint: ${mint.toBase58()}. ` +
                        `Please create a token pool via createTokenPool().`,
                );
            }

            instructions.push(
                createWrapInstruction(
                    splT22Ata,
                    ctokenAta,
                    owner,
                    mint,
                    balance,
                    tokenPoolInfo,
                    payer,
                ),
            );

            sources.push({
                type: mintTokenProgram.equals(TOKEN_PROGRAM_ID)
                    ? 'spl'
                    : 'token2022',
                address: splT22Ata,
                amount: balance,
            });
            totalAmount += balance;
        }
    }

    // 3. Decompress compressed tokens if they exist
    const compressedAccounts = compressedTokensResult.items;
    if (compressedAccounts.length > 0) {
        const compressedBalance = compressedAccounts.reduce(
            (sum, acc) => sum + BigInt(acc.parsed.amount.toString()),
            BigInt(0),
        );

        if (compressedBalance > BigInt(0)) {
            // We need a validity proof for compressed accounts
            requiresProof = true;
            compressedAccountsForProof = compressedAccounts;

            sources.push({
                type: 'compressed',
                address: owner, // Compressed accounts are identified by owner
                amount: compressedBalance,
            });
            totalAmount += compressedBalance;

            // Note: The actual decompress instruction will be built after proof generation
            // This function returns the info needed to generate proof and build the instruction
        }
    }

    return {
        ctokenAta,
        instructions,
        sources,
        totalAmount,
        requiresProof,
        compressedAccounts: compressedAccountsForProof,
    };
}

/**
 * Build the decompress instruction for compressed tokens to CToken ATA.
 * Call this after generating the validity proof.
 *
 * @param payer                          Fee payer public key
 * @param owner                          Owner public key
 * @param mint                           Mint address
 * @param ctokenAta                      CToken ATA address
 * @param inputCompressedTokenAccounts   Compressed token accounts to decompress
 * @param amount                         Amount to decompress
 * @param recentValidityProof            Validity proof
 * @param recentInputStateRootIndices    Root indices
 * @param tokenPoolInfos                 Token pool infos
 */
export async function buildDecompressToCTokenInstruction(
    payer: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    ctokenAta: PublicKey,
    inputCompressedTokenAccounts: ParsedTokenAccount[],
    amount: bigint | BN,
    recentValidityProof: ValidityProof | CompressedProof | null,
    recentInputStateRootIndices: number[],
    tokenPoolInfos: TokenPoolInfo[],
): Promise<TransactionInstruction> {
    // Use the standard decompress instruction but target CToken ATA
    // The on-chain routing will detect the CToken account owner and use CToken decompression
    const ix = await CompressedTokenProgram.decompress({
        payer,
        inputCompressedTokenAccounts,
        toAddress: ctokenAta,
        amount: bn(amount.toString()),
        recentValidityProof,
        recentInputStateRootIndices,
        tokenPoolInfos,
    });

    return ix;
}

/**
 * Load all token balances into a single CToken ATA.
 *
 * This action:
 * 1. Creates CToken ATA if it doesn't exist (idempotent)
 * 2. Wraps SPL/T22 tokens to CToken ATA if SPL/T22 ATA has balance
 * 3. Decompresses compressed tokens to CToken ATA if compressed tokens exist
 *
 * After this operation, all tokens for the given mint will be in the CToken ATA.
 *
 * @param rpc             RPC connection
 * @param payer           Fee payer
 * @param mint            Mint address
 * @param owner           Owner (must sign)
 * @param confirmOptions  Optional: Confirm options
 * @param options         Optional: Load-specific options (mintProgramId, tokenPoolInfos, outputStateTreeInfo)
 * @returns Result including CToken ATA address and transaction signature
 */
export async function loadAtaInterface(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    owner: Signer,
    confirmOptions?: ConfirmOptions,
    options?: LoadAtaOptions,
): Promise<LoadAtaInterfaceResult> {
    const {
        mintProgramId,
        tokenPoolInfos: providedTokenPoolInfos,
        outputStateTreeInfo: providedStateTreeInfo,
    } = options ?? {};

    // Build initial instructions
    const result = await loadAtaInterfaceInstructions(
        rpc,
        payer.publicKey,
        mint,
        owner.publicKey,
        {
            mintProgramId,
            tokenPoolInfos: providedTokenPoolInfos,
            outputStateTreeInfo: providedStateTreeInfo,
        },
    );

    const instructions = [...result.instructions];

    // If there are compressed tokens, generate proof and add decompress instruction
    if (result.requiresProof && result.compressedAccounts) {
        const compressedAccounts = result.compressedAccounts;
        const compressedBalance = compressedAccounts.reduce(
            (sum, acc) => sum + BigInt(acc.parsed.amount.toString()),
            BigInt(0),
        );

        // Get validity proof
        const proof = await rpc.getValidityProofV0(
            compressedAccounts.map(acc => ({
                hash: acc.compressedAccount.hash,
                tree: acc.compressedAccount.treeInfo.tree,
                queue: acc.compressedAccount.treeInfo.queue,
            })),
        );

        // Get token pool infos for decompress
        const tokenPoolInfos =
            providedTokenPoolInfos ?? (await getTokenPoolInfos(rpc, mint));
        const selectedPoolInfos = selectTokenPoolInfosForDecompression(
            tokenPoolInfos,
            bn(compressedBalance.toString()),
        );

        // Build decompress instruction
        const decompressIx = await buildDecompressToCTokenInstruction(
            payer.publicKey,
            owner.publicKey,
            mint,
            result.ctokenAta,
            compressedAccounts,
            compressedBalance,
            proof.compressedProof,
            proof.rootIndices,
            selectedPoolInfos,
        );

        instructions.push(decompressIx);
    }

    // Nothing to do if no sources
    if (result.sources.length === 0 && instructions.length === 0) {
        throw new Error(
            `No tokens found to load for owner ${owner.publicKey.toBase58()} and mint ${mint.toBase58()}`,
        );
    }

    // If we only have the create ATA instruction and no sources, just create the ATA
    if (result.sources.length === 0 && instructions.length === 1) {
        // Only creating ATA, no tokens to load
    }

    // Build and send transaction
    const { blockhash } = await rpc.getLatestBlockhash();

    // Determine additional signers
    const additionalSigners = dedupeSigner(payer, [owner]);

    const tx = buildAndSignTx(
        [
            ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }),
            ...instructions,
        ],
        payer,
        blockhash,
        additionalSigners,
    );

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return {
        ctokenAta: result.ctokenAta,
        transactionSignature: txId,
        sources: result.sources,
        totalAmount: result.totalAmount,
    };
}

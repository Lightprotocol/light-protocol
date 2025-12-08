import {
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
    ComputeBudgetProgram,
} from '@solana/web3.js';
import {
    Rpc,
    buildAndSignTx,
    sendAndConfirmTx,
    dedupeSigner,
    ParsedTokenAccount,
} from '@lightprotocol/stateless.js';
import {
    createAssociatedTokenAccountIdempotentInstruction,
    getAssociatedTokenAddress,
} from '@solana/spl-token';
import BN from 'bn.js';
import { createDecompressInterfaceInstruction } from '../instructions/create-decompress-interface-instruction';
import { createAssociatedTokenAccountInterfaceIdempotentInstruction } from '../instructions/create-ata-interface';
import { getAssociatedTokenAddressInterface } from '../get-associated-token-address-interface';
import { CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { SplInterfaceInfo } from '../../utils/get-token-pool-infos';

/**
 * Decompress compressed (cold) tokens to an on-chain token account.
 *
 * For unified loading, use {@link loadAta} instead.
 *
 * @param rpc                  RPC connection
 * @param payer                Fee payer (signer)
 * @param owner                Owner of the compressed tokens (signer)
 * @param mint                 Mint address
 * @param amount               Amount to decompress (defaults to all)
 * @param destinationAta       Destination token account address
 * @param destinationOwner     Owner of the destination ATA
 * @param splInterfaceInfo     SPL interface info for SPL/T22 destinations
 * @param confirmOptions       Confirm options
 * @returns Transaction signature, null if nothing to load.
 */
export async function decompressInterface(
    rpc: Rpc,
    payer: Signer,
    owner: Signer,
    mint: PublicKey,
    amount?: number | bigint | BN,
    destinationAta?: PublicKey,
    destinationOwner?: PublicKey,
    splInterfaceInfo?: SplInterfaceInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature | null> {
    // Determine if this is SPL or c-token destination
    const isSplDestination = splInterfaceInfo !== undefined;

    // Get compressed token accounts
    const compressedResult = await rpc.getCompressedTokenAccountsByOwner(
        owner.publicKey,
        { mint },
    );
    const compressedAccounts = compressedResult.items;

    if (compressedAccounts.length === 0) {
        return null; // Nothing to decompress
    }

    // Calculate total and determine amount
    const totalBalance = compressedAccounts.reduce(
        (sum, acc) => sum + BigInt(acc.parsed.amount.toString()),
        BigInt(0),
    );

    const decompressAmount = amount ? BigInt(amount.toString()) : totalBalance;

    if (decompressAmount > totalBalance) {
        throw new Error(
            `Insufficient compressed balance. Requested: ${decompressAmount}, Available: ${totalBalance}`,
        );
    }

    // Select accounts to use (for now, use all - could optimize later)
    const accountsToUse: ParsedTokenAccount[] = [];
    let accumulatedAmount = BigInt(0);
    for (const acc of compressedAccounts) {
        if (accumulatedAmount >= decompressAmount) break;
        accountsToUse.push(acc);
        accumulatedAmount += BigInt(acc.parsed.amount.toString());
    }

    // Get validity proof
    const validityProof = await rpc.getValidityProofV0(
        accountsToUse.map(acc => ({
            hash: acc.compressedAccount.hash,
            tree: acc.compressedAccount.treeInfo.tree,
            queue: acc.compressedAccount.treeInfo.queue,
        })),
    );

    // Determine destination ATA based on token program
    const ataOwner = destinationOwner ?? owner.publicKey;
    let destinationAtaAddress: PublicKey;

    if (isSplDestination) {
        // SPL destination - use SPL ATA
        destinationAtaAddress =
            destinationAta ??
            (await getAssociatedTokenAddress(
                mint,
                ataOwner,
                false,
                splInterfaceInfo.tokenProgram,
            ));
    } else {
        // c-token destination - use c-token ATA
        destinationAtaAddress =
            destinationAta ??
            getAssociatedTokenAddressInterface(mint, ataOwner);
    }

    // Build instructions
    const instructions = [];

    // Create ATA if needed (idempotent)
    const ataInfo = await rpc.getAccountInfo(destinationAtaAddress);
    if (!ataInfo) {
        if (isSplDestination) {
            // Create SPL ATA
            instructions.push(
                createAssociatedTokenAccountIdempotentInstruction(
                    payer.publicKey,
                    destinationAtaAddress,
                    ataOwner,
                    mint,
                    splInterfaceInfo.tokenProgram,
                ),
            );
        } else {
            // Create c-token ATA
            instructions.push(
                createAssociatedTokenAccountInterfaceIdempotentInstruction(
                    payer.publicKey,
                    destinationAtaAddress,
                    ataOwner,
                    mint,
                    CTOKEN_PROGRAM_ID,
                ),
            );
        }
    }

    // Calculate compute units
    const hasValidityProof = validityProof.compressedProof !== null;
    let computeUnits = 50_000; // Base
    if (hasValidityProof) {
        computeUnits += 100_000;
    }
    for (const acc of accountsToUse) {
        const proveByIndex = acc.compressedAccount.proveByIndex ?? false;
        computeUnits += proveByIndex ? 10_000 : 30_000;
    }
    // SPL decompression needs extra compute for pool operations
    if (isSplDestination) {
        computeUnits += 50_000;
    }

    // Add decompressInterface instruction
    instructions.push(
        createDecompressInterfaceInstruction(
            payer.publicKey,
            accountsToUse,
            destinationAtaAddress,
            decompressAmount,
            validityProof,
            splInterfaceInfo,
        ),
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

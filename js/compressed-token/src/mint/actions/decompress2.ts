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
    bn,
} from '@lightprotocol/stateless.js';
import BN from 'bn.js';
import { createDecompress2Instruction } from '../instructions/decompress2';
import { createAssociatedTokenAccountInterfaceIdempotentInstruction } from '../instructions/create-associated-ctoken';
import { getAtaAddressInterface } from './create-ata-interface';
import { CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';

/**
 * Parameters for decompress2 action
 */
export interface Decompress2ActionParams {
    /** RPC connection */
    rpc: Rpc;
    /** Fee payer (signer) */
    payer: Signer;
    /** Owner of the compressed tokens (signer) */
    owner: Signer;
    /** Mint address */
    mint: PublicKey;
    /** Optional: specific amount to decompress (defaults to all) */
    amount?: number | bigint | BN;
    /** Optional: destination CToken ATA (defaults to owner's ATA) */
    destinationAta?: PublicKey;
    /** Optional: confirm options */
    confirmOptions?: ConfirmOptions;
}

/**
 * Decompress compressed tokens to a CToken ATA using Transfer2.
 *
 * This is more efficient than the old decompress for CToken destinations
 * as it doesn't require SPL token pool operations.
 *
 * @param params Decompress2 action parameters
 * @returns Transaction signature, or null if no compressed tokens to decompress
 */
export async function decompress2(
    params: Decompress2ActionParams,
): Promise<TransactionSignature | null> {
    const {
        rpc,
        payer,
        owner,
        mint,
        amount: requestedAmount,
        destinationAta,
        confirmOptions,
    } = params;

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

    const amount = requestedAmount
        ? BigInt(requestedAmount.toString())
        : totalBalance;

    if (amount > totalBalance) {
        throw new Error(
            `Insufficient compressed balance. Requested: ${amount}, Available: ${totalBalance}`,
        );
    }

    // Select accounts to use (for now, use all - could optimize later)
    const accountsToUse: ParsedTokenAccount[] = [];
    let accumulatedAmount = BigInt(0);
    for (const acc of compressedAccounts) {
        if (accumulatedAmount >= amount) break;
        accountsToUse.push(acc);
        accumulatedAmount += BigInt(acc.parsed.amount.toString());
    }

    // Get validity proof
    const proof = await rpc.getValidityProofV0(
        accountsToUse.map(acc => ({
            hash: acc.compressedAccount.hash,
            tree: acc.compressedAccount.treeInfo.tree,
            queue: acc.compressedAccount.treeInfo.queue,
        })),
    );

    // Determine destination ATA
    const ctokenAta =
        destinationAta ?? getAtaAddressInterface(mint, owner.publicKey);

    // Build instructions
    const instructions = [];

    // Create CToken ATA if needed (idempotent)
    const ctokenAtaInfo = await rpc.getAccountInfo(ctokenAta);
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

    // Calculate compute units
    const hasValidityProof = proof.compressedProof !== null;
    let computeUnits = 50_000; // Base
    if (hasValidityProof) {
        computeUnits += 100_000;
    }
    for (const acc of accountsToUse) {
        const proveByIndex = acc.compressedAccount.proveByIndex ?? false;
        computeUnits += proveByIndex ? 10_000 : 30_000;
    }

    // Add decompress2 instruction
    instructions.push(
        createDecompress2Instruction(
            payer.publicKey,
            accountsToUse,
            ctokenAta,
            amount,
            proof.compressedProof,
            proof.rootIndices,
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

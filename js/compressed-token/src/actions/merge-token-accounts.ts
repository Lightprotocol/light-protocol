import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import {
    Rpc,
    dedupeSigner,
    buildAndSignTx,
    sendAndConfirmTx,
    bn,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../program';

/**
 * Merge multiple compressed token accounts for a given mint into fewer
 * accounts. Each call merges up to 4 accounts at a time. Call repeatedly
 * until only 1 account remains if full consolidation is needed.
 *
 * @param rpc                   RPC connection to use
 * @param payer                 Fee payer
 * @param mint                  SPL Mint address
 * @param owner                 Owner of the token accounts to be merged
 * @param confirmOptions        Options for confirming the transaction
 *
 * @return confirmed transaction signature
 */
export async function mergeTokenAccounts(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    owner: Signer,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const compressedTokenAccounts = await rpc.getCompressedTokenAccountsByOwner(
        owner.publicKey,
        { mint },
    );

    if (compressedTokenAccounts.items.length === 0) {
        throw new Error(
            `No compressed token accounts found for mint ${mint.toBase58()}`,
        );
    }

    if (compressedTokenAccounts.items.length === 1) {
        throw new Error('Only one token account exists, nothing to merge');
    }

    // Take up to 4 accounts to merge in this transaction
    const batch = compressedTokenAccounts.items.slice(0, 4);

    const proof = await rpc.getValidityProof(
        batch.map(account => bn(account.compressedAccount.hash)),
    );

    const mergeInstructions = await CompressedTokenProgram.mergeTokenAccounts({
        payer: payer.publicKey,
        owner: owner.publicKey,
        inputCompressedTokenAccounts: batch,
        mint,
        recentValidityProof: proof.compressedProof,
        recentInputStateRootIndices: proof.rootIndices,
    });

    const instructions = [
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }),
        ...mergeInstructions,
    ];

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);

    const signedTx = buildAndSignTx(
        instructions,
        payer,
        blockhash,
        additionalSigners,
    );

    return sendAndConfirmTx(rpc, signedTx, confirmOptions);
}

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
 * Merge multiple compressed token accounts for a given mint into a single
 * account
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

    const instructions = [
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }),
    ];

    for (
        let i = 0;
        i < compressedTokenAccounts.items.slice(0, 8).length;
        i += 4
    ) {
        const batch = compressedTokenAccounts.items.slice(i, i + 4);

        const proof = await rpc.getValidityProof(
            batch.map(account => bn(account.compressedAccount.hash)),
        );

        const batchInstructions =
            await CompressedTokenProgram.mergeTokenAccounts({
                payer: payer.publicKey,
                owner: owner.publicKey,
                inputCompressedTokenAccounts: batch,
                mint,
                recentValidityProof: proof.compressedProof,
                recentInputStateRootIndices: proof.rootIndices,
            });

        instructions.push(...batchInstructions);
    }

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

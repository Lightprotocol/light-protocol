import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import {
    Rpc,
    ParsedTokenAccount,
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
 * @param rpc             RPC to use
 * @param payer           Payer of the transaction fees
 * @param mint            Public key of the token's mint
 * @param owner           Owner of the token accounts to be merged
 * @param merkleTree      Optional merkle tree for compressed tokens
 * @param confirmOptions  Options for confirming the transaction
 *
 * @return Array of transaction signatures
 */
export async function mergeTokenAccounts(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    owner: Signer,
    merkleTree?: PublicKey,
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
    if (compressedTokenAccounts.items.length >= 6) {
        throw new Error(
            `Too many compressed token accounts used for mint ${mint.toBase58()}`,
        );
    }

    const instructions = [
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }),
    ];

    for (
        let i = 0;
        i < compressedTokenAccounts.items.slice(0, 6).length;
        i += 3
    ) {
        const batch = compressedTokenAccounts.items.slice(i, i + 3);

        const proof = await rpc.getValidityProof(
            batch.map(account => bn(account.compressedAccount.hash)),
        );

        const batchInstructions =
            await CompressedTokenProgram.mergeTokenAccounts({
                payer: payer.publicKey,
                owner: owner.publicKey,
                mint,
                inputCompressedTokenAccounts: batch,
                outputStateTree: merkleTree!,
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
    const txId = await sendAndConfirmTx(rpc, signedTx, confirmOptions);

    return txId;
}

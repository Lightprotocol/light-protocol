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
    StateTreeInfo,
    pickStateTreeInfo,
    TreeType,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../program';

/**
 * Merge multiple compressed token accounts for a given mint into a single
 * account
 *
 * @param rpc                       RPC to use
 * @param payer                     Payer of the transaction fees
 * @param mint                      Public key of the token's mint
 * @param owner                     Owner of the token accounts to be merged
 * @param outputStateTreeInfo    State tree context that the compressed
 *                                  tokens should be part of. Defaults to the
 *                                  default state tree context.
 * @param confirmOptions            Options for confirming the transaction
 *
 * @return Array of transaction signatures
 */
export async function mergeTokenAccounts(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    owner: Signer,
    outputStateTreeInfo?: StateTreeInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    if (!outputStateTreeInfo) {
        const stateTreeInfo = await rpc.getCachedActiveStateTreeInfos();
        outputStateTreeInfo = pickStateTreeInfo(
            stateTreeInfo,
            TreeType.StateV2,
        );
    }

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
                outputStateTreeInfo: outputStateTreeInfo!,
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

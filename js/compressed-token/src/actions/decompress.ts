import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import {
    bn,
    sendAndConfirmTx,
    buildAndSignTx,
    Rpc,
    dedupeSigner,
} from '@lightprotocol/stateless.js';
import BN from 'bn.js';
import { CompressedTokenProgram } from '../program';
import {
    selectMinCompressedTokenAccountsForTransfer,
    selectAccountsByPreferredTreeType,
} from '../utils';
import {
    selectSplInterfaceInfosForDecompression,
    SplInterfaceInfo,
    getSplInterfaceInfos,
} from '../utils/get-token-pool-infos';

/**
 * Decompress compressed tokens
 *
 * @param rpc                   Rpc connection to use
 * @param payer                 Fee payer
 * @param mint                  SPL Mint address
 * @param amount                Number of tokens to transfer
 * @param owner                 Owner of the compressed tokens
 * @param toAddress             Destination **uncompressed** token account
 *                              address. (ATA)
 * @param splInterfaceInfos     Optional: SPL interface infos.
 * @param confirmOptions        Options for confirming the transaction
 *
 * @return confirmed transaction signature
 */
export async function decompress(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    amount: number | BN,
    owner: Signer,
    toAddress: PublicKey,
    splInterfaceInfos?: SplInterfaceInfo[],
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    amount = bn(amount);

    const compressedTokenAccounts = await rpc.getCompressedTokenAccountsByOwner(
        owner.publicKey,
        { mint },
    );

    // Select accounts from preferred tree type (V2 in V2 mode) with fallback
    const { accounts: accountsToUse } = selectAccountsByPreferredTreeType(
        compressedTokenAccounts.items,
        amount,
    );

    const [inputAccounts] = selectMinCompressedTokenAccountsForTransfer(
        accountsToUse,
        amount,
    );

    const proof = await rpc.getValidityProofV0(
        inputAccounts.map(account => ({
            hash: account.compressedAccount.hash,
            tree: account.compressedAccount.treeInfo.tree,
            queue: account.compressedAccount.treeInfo.queue,
        })),
    );

    splInterfaceInfos =
        splInterfaceInfos ?? (await getSplInterfaceInfos(rpc, mint));

    const selectedSplInterfaceInfos = selectSplInterfaceInfosForDecompression(
        splInterfaceInfos,
        amount,
    );

    const ix = await CompressedTokenProgram.decompress({
        payer: payer.publicKey,
        inputCompressedTokenAccounts: inputAccounts,
        toAddress,
        amount,
        tokenPoolInfos: selectedSplInterfaceInfos,
        recentInputStateRootIndices: proof.rootIndices,
        recentValidityProof: proof.compressedProof,
    });

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);
    const signedTx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 350_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );
    return await sendAndConfirmTx(rpc, signedTx, confirmOptions);
}

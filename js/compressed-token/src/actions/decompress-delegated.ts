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
    TreeType,
    featureFlags,
} from '@lightprotocol/stateless.js';

import BN from 'bn.js';

import { CompressedTokenProgram } from '../program';
import {
    selectMinCompressedTokenAccountsForTransfer,
    groupAccountsByTreeType,
} from '../utils';
import {
    selectSplInterfaceInfosForDecompression,
    SplInterfaceInfo,
    getSplInterfaceInfos,
} from '../utils/get-token-pool-infos';

/**
 * Decompress delegated compressed tokens. Remaining compressed tokens are
 * returned to the owner without delegation.
 *
 * @param rpc                   Rpc connection to use
 * @param payer                 Fee payer
 * @param mint                  SPL Mint address
 * @param amount                Number of tokens to decompress
 * @param owner                 Owner of the compressed tokens
 * @param toAddress             Destination **uncompressed** token account
 *                              address. (ATA)
 * @param splInterfaceInfos     Optional: SPL interface infos.
 * @param confirmOptions        Options for confirming the transaction
 *
 * @return Signature of the confirmed transaction
 */
export async function decompressDelegated(
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

    const compressedTokenAccounts =
        await rpc.getCompressedTokenAccountsByDelegate(owner.publicKey, {
            mint,
        });

    // Prefer inputs matching SDK mode (V2 by default), fall back if insufficient
    const preferredTreeType = featureFlags.isV2()
        ? TreeType.StateV2
        : TreeType.StateV1;

    const accountsByTreeType = groupAccountsByTreeType(
        compressedTokenAccounts.items,
    );

    let accountsToUse = accountsByTreeType.get(preferredTreeType) || [];

    const preferredBalance = accountsToUse.reduce(
        (sum, acc) => sum.add(acc.parsed.amount),
        bn(0),
    );

    if (preferredBalance.lt(amount)) {
        const fallbackType =
            preferredTreeType === TreeType.StateV2
                ? TreeType.StateV1
                : TreeType.StateV2;
        const fallbackAccounts = accountsByTreeType.get(fallbackType) || [];
        const fallbackBalance = fallbackAccounts.reduce(
            (sum, acc) => sum.add(acc.parsed.amount),
            bn(0),
        );

        if (fallbackBalance.gte(amount)) {
            accountsToUse = fallbackAccounts;
        }
    }

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

    const splInterfaceInfosToUse =
        splInterfaceInfos ??
        selectSplInterfaceInfosForDecompression(
            await getSplInterfaceInfos(rpc, mint),
            amount,
        );

    const ix = await CompressedTokenProgram.decompress({
        payer: payer.publicKey,
        inputCompressedTokenAccounts: inputAccounts,
        toAddress,
        amount,
        recentInputStateRootIndices: proof.rootIndices,
        recentValidityProof: proof.compressedProof,
        tokenPoolInfos: splInterfaceInfosToUse,
    });

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);
    const signedTx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 350_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    return sendAndConfirmTx(rpc, signedTx, confirmOptions);
}

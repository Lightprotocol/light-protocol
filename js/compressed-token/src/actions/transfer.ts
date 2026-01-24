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

/**
 * Transfer compressed tokens from one owner to another.
 *
 * Supports automatic V1 -> V2 migration: when running in V2 mode,
 * V1 token inputs will produce V2 token outputs.
 *
 * @param rpc                   Rpc connection to use
 * @param payer                 Fee payer
 * @param mint                  SPL Mint address
 * @param amount                Number of tokens to transfer
 * @param owner                 Owner of the compressed tokens
 * @param toAddress             Destination address of the recipient
 * @param confirmOptions        Options for confirming the transaction
 *
 * @return confirmed transaction signature
 */
export async function transfer(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    amount: number | BN,
    owner: Signer,
    toAddress: PublicKey,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    amount = bn(amount);

    const compressedTokenAccounts = await rpc.getCompressedTokenAccountsByOwner(
        owner.publicKey,
        { mint },
    );

    // Prefer inputs matching SDK mode (V2 by default), fall back if insufficient
    const isV2Mode = featureFlags.isV2();
    const preferredTreeType = isV2Mode ? TreeType.StateV2 : TreeType.StateV1;

    // Group accounts by tree type to ensure consistent selection
    const accountsByTreeType = groupAccountsByTreeType(
        compressedTokenAccounts.items,
    );

    // Try to select from preferred tree type first
    let selectedTreeType = preferredTreeType;
    let accountsToUse = accountsByTreeType.get(preferredTreeType) || [];

    // If insufficient balance in preferred type, fall back to other type
    const preferredBalance = accountsToUse.reduce(
        (sum, acc) => sum.add(acc.parsed.amount),
        bn(0),
    );

    if (preferredBalance.lt(amount)) {
        // Try the other tree type
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
            selectedTreeType = fallbackType;
            accountsToUse = fallbackAccounts;
        }
        // If neither type has enough, proceed with preferred type
        // and let selectMinCompressedTokenAccountsForTransfer throw
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

    // V1→V2 migration handled inside CompressedTokenProgram.transfer
    const ix = await CompressedTokenProgram.transfer({
        payer: payer.publicKey,
        inputCompressedTokenAccounts: inputAccounts,
        toAddress,
        amount,
        recentInputStateRootIndices: proof.rootIndices,
        recentValidityProof: proof.compressedProof,
    });

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);
    const signedTx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    return sendAndConfirmTx(rpc, signedTx, confirmOptions);
}

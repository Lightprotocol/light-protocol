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
    TreeType,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../program';
import { selectAccountsByPreferredTreeType } from '../utils';

/**
 * Max input accounts per merge.
 *
 * Even though V2 supports larger merges, we keep this at 4 to avoid oversized
 * transactions / RPC payload limits under heavy test load.
 */
const MAX_MERGE_ACCOUNTS = 4;

/**
 * Merge multiple compressed token accounts for a given mint into fewer
 * accounts. Each call merges up to 4 accounts at a time.
 *
 * Supports automatic V1 -> V2 migration: when running in V2 mode,
 * merging V1 token accounts will produce a V2 output.
 *
 * IMPORTANT: Only accounts from the same tree type can be merged in one
 * transaction. If you have mixed V1+V2 accounts, merge them separately.
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

    // Select accounts from preferred tree type (V2 in V2 mode) - for merge need at least 2
    const { accounts: preferredAccounts, treeType: preferredTreeType } =
        selectAccountsByPreferredTreeType(compressedTokenAccounts.items);

    let selectedAccounts = preferredAccounts;
    let selectedTreeType = preferredTreeType;

    // For merge, need at least 2 accounts of the same type
    // If preferred type has < 2, try fallback type
    if (selectedAccounts.length < 2) {
        const fallbackType =
            preferredTreeType === TreeType.StateV2
                ? TreeType.StateV1
                : TreeType.StateV2;
        const fallbackAccounts = compressedTokenAccounts.items.filter(
            acc => acc.compressedAccount.treeInfo.treeType === fallbackType,
        );

        if (fallbackAccounts.length >= 2) {
            selectedAccounts = fallbackAccounts;
            selectedTreeType = fallbackType;
        } else if (
            selectedAccounts.length === 1 &&
            fallbackAccounts.length === 1
        ) {
            // Have 1 V1 and 1 V2 - can't merge mixed types
            throw new Error(
                'Cannot merge accounts from different tree types (V1/V2). ' +
                    'You have 1 V1 and 1 V2 account - nothing to merge within same type.',
            );
        } else {
            throw new Error(
                `Not enough accounts of the same tree type to merge. ` +
                    `Found: ${selectedAccounts.length} ${selectedTreeType === TreeType.StateV1 ? 'V1' : 'V2'} accounts.`,
            );
        }
    }

    // Take up to MAX_MERGE_ACCOUNTS to merge in this transaction
    const batch = selectedAccounts.slice(0, MAX_MERGE_ACCOUNTS);

    const proof = await rpc.getValidityProof(
        batch.map(account => bn(account.compressedAccount.hash)),
    );

    // V1→V2 migration handled inside CompressedTokenProgram.mergeTokenAccounts
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

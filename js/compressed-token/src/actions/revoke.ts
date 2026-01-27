import {
    ComputeBudgetProgram,
    ConfirmOptions,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import {
    sendAndConfirmTx,
    buildAndSignTx,
    Rpc,
    dedupeSigner,
    ParsedTokenAccount,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../program';
import { groupAccountsByTreeType } from '../utils';

/**
 * Revoke one or more delegated token accounts
 *
 * @param rpc                   Rpc connection to use
 * @param payer                 Fee payer
 * @param accounts              Delegated compressed token accounts to revoke
 *                              (must all be from the same tree type)
 * @param owner                 Owner of the compressed tokens
 * @param confirmOptions        Options for confirming the transaction
 *
 * @return Signature of the confirmed transaction
 */
export async function revoke(
    rpc: Rpc,
    payer: Signer,
    accounts: ParsedTokenAccount[],
    owner: Signer,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    // Validate all accounts are from the same tree type
    const accountsByTreeType = groupAccountsByTreeType(accounts);
    if (accountsByTreeType.size > 1) {
        throw new Error(
            'Cannot revoke accounts from different tree types (V1/V2) in the same transaction. ' +
                'Revoke V1 and V2 accounts separately.',
        );
    }

    const proof = await rpc.getValidityProofV0(
        accounts.map(account => ({
            hash: account.compressedAccount.hash,
            tree: account.compressedAccount.treeInfo.tree,
            queue: account.compressedAccount.treeInfo.queue,
        })),
    );
    checkOwner(owner, accounts);
    checkIsDelegated(accounts);

    const ix = await CompressedTokenProgram.revoke({
        payer: payer.publicKey,
        inputCompressedTokenAccounts: accounts,
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

function checkOwner(owner: Signer, accounts: ParsedTokenAccount[]) {
    if (!owner.publicKey.equals(accounts[0].parsed.owner)) {
        throw new Error(
            `Owner ${owner.publicKey.toBase58()} does not match account ${accounts[0].parsed.owner.toBase58()}`,
        );
    }
}

function checkIsDelegated(accounts: ParsedTokenAccount[]) {
    if (accounts.some(account => account.parsed.delegate === null)) {
        throw new Error('Account is not delegated');
    }
}

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

/**
 * Revoke one or more delegated token accounts
 *
 * @param rpc                   Rpc connection to use
 * @param payer                 Fee payer
 * @param accounts              Delegated compressed token accounts to revoke
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

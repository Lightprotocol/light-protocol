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
    selectTokenAccountsForApprove,
} from '../utils';

/**
 * Approve a delegate to spend tokens
 *
 * @param rpc                   Rpc to use
 * @param payer                 Fee payer
 * @param mint                  SPL Mint address
 * @param amount                Number of tokens to delegate
 * @param owner                 Owner of the SPL token account.
 * @param delegate              Address of the delegate
 * @param confirmOptions        Options for confirming the transaction
 *
 * @return Signature of the confirmed transaction
 */
export async function approve(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    amount: number | BN,
    owner: Signer,
    delegate: PublicKey,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    amount = bn(amount);
    const compressedTokenAccounts = await rpc.getCompressedTokenAccountsByOwner(
        owner.publicKey,
        {
            mint,
        },
    );

    const [inputAccounts] = selectTokenAccountsForApprove(
        compressedTokenAccounts.items,
        amount,
    );

    const proof = await rpc.getValidityProofV0(
        inputAccounts.map(account => ({
            hash: account.compressedAccount.hash,
            tree: account.compressedAccount.treeInfo.tree,
            queue: account.compressedAccount.treeInfo.queue,
        })),
    );

    const ix = await CompressedTokenProgram.approve({
        payer: payer.publicKey,
        inputCompressedTokenAccounts: inputAccounts,
        toAddress: delegate,
        amount,
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

    return sendAndConfirmTx(rpc, signedTx, confirmOptions);
}

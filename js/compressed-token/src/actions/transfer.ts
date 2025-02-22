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
    StateTreeContext,
    pickRandomStateTreeContext,
} from '@lightprotocol/stateless.js';

import BN from 'bn.js';

import { CompressedTokenProgram } from '../program';
import { selectMinCompressedTokenAccountsForTransfer } from '../utils';

/**
 * Transfer compressed tokens from one owner to another
 *
 * @param rpc                       Rpc to use
 * @param payer                     Payer of the transaction fees
 * @param mint                      Mint of the compressed token
 * @param amount                    Number of tokens to transfer
 * @param owner                     Owner of the compressed tokens
 * @param toAddress                 Destination address of the recipient
 * @param outputStateTreeContext    State tree context that the compressed
 *                                  tokens should be inserted into. Defaults to
 *                                  the default state tree context.
 * @param confirmOptions            Options for confirming the transaction
 *
 * @return Signature of the confirmed transaction
 */
export async function transfer(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    amount: number | BN,
    owner: Signer,
    toAddress: PublicKey,
    outputStateTreeContext?: StateTreeContext,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    amount = bn(amount);

    if (!outputStateTreeContext) {
        const stateTreeInfo = await rpc.getCachedActiveStateTreeInfo();
        outputStateTreeContext = pickRandomStateTreeContext(stateTreeInfo);
    }

    const compressedTokenAccounts = await rpc.getCompressedTokenAccountsByOwner(
        owner.publicKey,
        {
            mint,
        },
    );

    const [inputAccounts] = selectMinCompressedTokenAccountsForTransfer(
        compressedTokenAccounts.items,
        amount,
    );

    const proof = await rpc.getValidityProof(
        inputAccounts.map(account => bn(account.compressedAccount.hash)),
    );

    const ix = await CompressedTokenProgram.transfer({
        payer: payer.publicKey,
        inputCompressedTokenAccounts: inputAccounts,
        toAddress,
        amount,
        recentInputStateRootIndices: proof.rootIndices,
        recentValidityProof: proof.compressedProof,
        outputStateTreeContext,
    });

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);
    const signedTx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    const txId = await sendAndConfirmTx(rpc, signedTx, confirmOptions);
    return txId;
}

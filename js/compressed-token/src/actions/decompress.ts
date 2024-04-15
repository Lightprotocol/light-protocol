import {
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import {
    bn,
    defaultTestStateTreeAccounts,
    sendAndConfirmTx,
    buildAndSignTx,
    Rpc,
    dedupeSigner,
} from '@lightprotocol/stateless.js';

import { BN } from '@coral-xyz/anchor';

import { CompressedTokenProgram } from '../program';
import { selectMinCompressedTokenAccountsForTransfer } from './transfer';

/**
 * Decompress compressed tokens
 *
 * @param rpc            Rpc to use
 * @param payer          Payer of the transaction fees
 * @param mint           Mint of the compressed token
 * @param amount         Number of tokens to transfer
 * @param owner          Owner of the compressed tokens
 * @param toAddress      Destination **uncompressed** (associated) token account
 *                       address.
 * @param merkleTree     State tree account that any change compressed tokens should be
 *                       inserted into. Defaults to a default state tree
 *                       account.
 * @param confirmOptions Options for confirming the transaction
 *
 *
 * @return Signature of the confirmed transaction
 */
export async function decompress(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    amount: number | BN,
    owner: Signer,
    toAddress: PublicKey,
    /// TODO: allow multiple
    merkleTree: PublicKey = defaultTestStateTreeAccounts().merkleTree,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    amount = bn(amount);

    const compressedTokenAccounts = await rpc.getCompressedTokenAccountsByOwner(
        owner.publicKey,
        {
            mint,
        },
    );

    /// TODO: consider using a different selection algorithm
    const [inputAccounts] = selectMinCompressedTokenAccountsForTransfer(
        compressedTokenAccounts,
        amount,
    );

    const proof = await rpc.getValidityProof(
        inputAccounts.map(account => bn(account.compressedAccount.hash)),
    );

    const ixs = await CompressedTokenProgram.decompress({
        payer: payer.publicKey,
        inputCompressedTokenAccounts: inputAccounts,
        toAddress, // TODO: add explicit check that it is a token account
        amount,
        outputStateTree: merkleTree,
        recentInputStateRootIndices: proof.rootIndices,
        recentValidityProof: proof.compressedProof,
    });

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);
    const signedTx = buildAndSignTx(ixs, payer, blockhash, additionalSigners);
    const txId = await sendAndConfirmTx(rpc, signedTx, confirmOptions);
    return txId;
}

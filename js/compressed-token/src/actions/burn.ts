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
import { selectMinCompressedTokenAccountsForTransfer } from './transfer';

/**
 * Decompress compressed tokens
 *
 * @param rpc            Rpc to use
 * @param payer          Payer of the transaction fees
 * @param mint           Mint of the compressed token
 * @param amount         Number of tokens to burn
 * @param owner          Owner of the compressed tokens
 * @param merkleTree     State tree account that any change compressed tokens should be
 *                       inserted into. Defaults to a default state tree
 *                       account.
 * @param confirmOptions Options for confirming the transaction
 * @param tokenProgramId Optional: The token program ID. Default: SPL Token Program ID
 *
 * @return Signature of the confirmed transaction
 */
export async function burn(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    amount: number | BN,
    owner: Signer,
    merkleTree?: PublicKey,
    confirmOptions?: ConfirmOptions,
    tokenProgramId?: PublicKey,
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
        compressedTokenAccounts.items,
        amount,
    );

    const proof = await rpc.getValidityProof(
        inputAccounts.map(account => bn(account.compressedAccount.hash)),
    );

    const ix = await CompressedTokenProgram.burn({
        payer: payer.publicKey,
        inputCompressedTokenAccounts: inputAccounts,
        amount,
        outputStateTree: merkleTree,
        recentInputStateRootIndices: proof.rootIndices,
        recentValidityProof: proof.compressedProof,
        tokenProgramId,
    });

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);
    const signedTx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );
    const txId = await sendAndConfirmTx(rpc, signedTx, confirmOptions);
    return txId;
}

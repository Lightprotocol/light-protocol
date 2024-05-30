import {
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
    ComputeBudgetProgram,
} from '@solana/web3.js';
import {
    bn,
    sendAndConfirmTx,
    buildAndSignTx,
    Rpc,
    dedupeSigner,
} from '@lightprotocol/stateless.js';

import { BN } from '@coral-xyz/anchor';

import { CompressedTokenProgram } from '../program';

/**
 * Compress SPL tokens
 *
 * @param rpc                   Rpc connection to use
 * @param payer                 Payer of the transaction fees
 * @param mint                  Mint of the compressed token
 * @param amount                Number of tokens to transfer
 * @param owner                 Owner of the compressed tokens.
 * @param sourceTokenAccount    Source (associated) token account
 * @param toAddress             Destination address of the recipient
 * @param merkleTree            State tree account that the compressed tokens
 *                              should be inserted into. Defaults to a default
 *                              state tree account.
 * @param confirmOptions        Options for confirming the transaction
 *
 *
 * @return Signature of the confirmed transaction
 */
export async function compress(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    amount: number | BN,
    owner: Signer,
    sourceTokenAccount: PublicKey,
    toAddress: PublicKey,
    merkleTree?: PublicKey,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    amount = bn(amount);

    const [approveIx, compressIx] = await CompressedTokenProgram.compress({
        payer: payer.publicKey,
        owner: owner.publicKey,
        source: sourceTokenAccount,
        toAddress,
        amount,
        mint,
        outputStateTree: merkleTree,
    });

    const blockhashCtx = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);
    const signedTx = buildAndSignTx(
        [
            ComputeBudgetProgram.setComputeUnitLimit({
                units: 1_000_000,
            }),
            approveIx,
            compressIx,
        ],
        payer,
        blockhashCtx.blockhash,
        additionalSigners,
    );
    const txId = await sendAndConfirmTx(
        rpc,
        signedTx,
        confirmOptions,
        blockhashCtx,
    );
    return txId;
}

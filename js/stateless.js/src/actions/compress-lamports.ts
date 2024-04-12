import {
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';

import { LightSystemProgram } from '../programs';
import { Rpc } from '../rpc';
import { buildAndSignTx, sendAndConfirmTx } from '../utils';
import { BN } from '@coral-xyz/anchor';
import { defaultTestStateTreeAccounts } from '../constants';

/**
 * Init the SOL omnibus account for Light
 *
 * @param rpc             RPC to use
 * @param payer           Payer of the transaction and initialization fees
 * @param lamports        Amount of lamports to compress
 * @param toAddress       Address of the recipient compressed account
 * @param outputStateTree Optional output state tree. Defaults to a current shared state tree.
 * @param confirmOptions  Options for confirming the transaction
 *
 * @return Transaction signature
 */
/// TODO: add multisig support
/// TODO: add support for payer != owner
export async function compressLamports(
    rpc: Rpc,
    payer: Signer,
    lamports: number | BN,
    toAddress: PublicKey,
    outputStateTree?: PublicKey,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const { blockhash } = await rpc.getLatestBlockhash();

    const ixs = await LightSystemProgram.compress({
        payer: payer.publicKey,
        toAddress,
        lamports,
        outputStateTree: outputStateTree
            ? outputStateTree
            : defaultTestStateTreeAccounts().merkleTree, // TODO: should fetch the current shared state tree
    });

    const tx = buildAndSignTx(ixs, payer, blockhash, []);

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return txId;
}

import {
    ComputeBudgetProgram,
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
 * Compress lamports to a solana address
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
export async function compress(
    rpc: Rpc,
    payer: Signer,
    lamports: number | BN,
    toAddress: PublicKey,
    outputStateTree?: PublicKey,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const { blockhash } = await rpc.getLatestBlockhash();

    const ix = await LightSystemProgram.compress({
        payer: payer.publicKey,
        toAddress,
        lamports,
        outputStateTree,
    });

    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }), ix],
        payer,
        blockhash,
        [],
    );

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return txId;
}

import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';

import { LightSystemProgram } from '../programs';
import { pickRandomStateTreeContext, Rpc } from '../rpc';
import { buildAndSignTx, sendAndConfirmTx } from '../utils';
import BN from 'bn.js';
import { StateTreeContext, TreeType } from '../state';

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
export async function compress(
    rpc: Rpc,
    payer: Signer,
    lamports: number | BN,
    toAddress: PublicKey,
    outputStateTreeContext?: StateTreeContext,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const { blockhash } = await rpc.getLatestBlockhash();

    if (!outputStateTreeContext) {
        const stateTreeInfo = await rpc.getCachedActiveStateTreeInfo();
        outputStateTreeContext = pickRandomStateTreeContext(
            stateTreeInfo,
            TreeType.BatchedState,
        );
    }

    const ix = await LightSystemProgram.compress({
        payer: payer.publicKey,
        toAddress,
        lamports,
        outputStateTreeContext,
    });

    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 600_000 }), ix],
        payer,
        blockhash,
        [],
    );

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return txId;
}

import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import { LightSystemProgram } from '../programs';
import { Rpc } from '../rpc';
import {
    buildAndSignTx,
    selectStateTreeInfo,
    sendAndConfirmTx,
} from '../utils';
import BN from 'bn.js';
import { TreeInfo } from '../state';

/**
 * Compress lamports to a solana address
 *
 * @param rpc                   RPC to use
 * @param payer                 Payer of the transaction and initialization fees
 * @param lamports              Amount of lamports to compress
 * @param toAddress             Address of the recipient compressed account
 * @param outputStateTreeInfo   Optional output state tree. If not provided,
 *                              fetches a random active state tree.
 * @param confirmOptions        Options for confirming the transaction
 *
 * @return Transaction signature
 */
export async function compress(
    rpc: Rpc,
    payer: Signer,
    lamports: number | BN,
    toAddress: PublicKey,
    outputStateTreeInfo?: TreeInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const { blockhash } = await rpc.getLatestBlockhash();

    if (!outputStateTreeInfo) {
        const stateTreeInfo = await rpc.getStateTreeInfos();
        outputStateTreeInfo = selectStateTreeInfo(stateTreeInfo);
    }

    const ix = await LightSystemProgram.compress({
        payer: payer.publicKey,
        toAddress,
        lamports,
        outputStateTreeInfo,
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

import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import { LightSystemProgram } from '../programs';
import { pickStateTreeInfo } from '../utils/get-light-state-tree-info';
import { buildAndSignTx, sendAndConfirmTx } from '../utils';
import BN from 'bn.js';
import { StateTreeInfo, TreeType } from '../state';
import { Rpc } from '../rpc';

/**
 * Compress lamports to a solana address
 *
 * @param rpc                   Connection to use
 * @param payer                 Payer of the transaction and initialization fees
 * @param lamports              Amount of lamports to compress
 * @param toAddress             Address of the recipient compressed account
 * @param outputStateTreeInfo   Optional output state tree info. Defaults to a
 *                              current shared state tree.
 * @param confirmOptions        Options for confirming the transaction
 *
 * @return Transaction signature
 */
export async function compress(
    rpc: Rpc,
    payer: Signer,
    lamports: number | BN,
    toAddress: PublicKey,
    outputStateTreeInfo?: StateTreeInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const { blockhash } = await rpc.getLatestBlockhash();

    if (!outputStateTreeInfo) {
        const stateTreeInfo = await rpc.getCachedActiveStateTreeInfos();
        outputStateTreeInfo = pickStateTreeInfo(
            stateTreeInfo,
            TreeType.StateV2,
        );
    }

    const ix = await LightSystemProgram.compress({
        payer: payer.publicKey,
        toAddress,
        lamports,
        outputStateTreeInfo,
    });

    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }), ix],
        payer,
        blockhash,
        [],
    );

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return txId;
}

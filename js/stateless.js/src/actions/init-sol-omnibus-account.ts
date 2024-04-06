import { ConfirmOptions, Signer, TransactionSignature } from '@solana/web3.js';

import { dedupeSigner } from './common';
import { LightSystemProgram } from '../programs';
import { Rpc } from '../rpc';
import { buildAndSignTx, sendAndConfirmTx } from '../utils';

/**
 * Init the SOL omnibus account for Light
 *
 * @param rpc             RPC to use
 * @param payer           Payer of the transaction and initialization fees
 * @param initAuthority   Init authority.
 * @param confirmOptions  Options for confirming the transaction
 *
 * @return Transaction signature
 */
/// TODO: add multisig support
export async function initSolOmnibusAccount(
    rpc: Rpc,
    payer: Signer,
    initAuthority?: Signer,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const { blockhash } = await rpc.getLatestBlockhash();

    const additionalSigners = dedupeSigner(
        payer,
        initAuthority ? [initAuthority] : [],
    );

    const ix = await LightSystemProgram.initCompressedSolPda(
        initAuthority ? initAuthority.publicKey : payer.publicKey,
    );

    const tx = buildAndSignTx([ix], payer, blockhash, additionalSigners);

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return txId;
}

import {
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import { LightSystemProgram, sumUpLamports } from '../programs';
import { Rpc } from '../rpc';
import { buildAndSignTx, sendAndConfirmTx } from '../utils';
import { BN } from '@coral-xyz/anchor';
import { CompressedAccountWithMerkleContext, bn } from '../state';

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
export async function decompressLamports(
    rpc: Rpc,
    payer: Signer,
    lamports: number | BN,
    recipient: PublicKey,
    outputStateTree?: PublicKey,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    /// TODO: use dynamic state tree and nullifier queue

    const userCompressedAccountsWithMerkleContext: CompressedAccountWithMerkleContext[] =
        await rpc.getCompressedAccountsByOwner(payer.publicKey);

    lamports = bn(lamports);

    const inputLamports = sumUpLamports(
        userCompressedAccountsWithMerkleContext,
    );

    if (lamports.gt(inputLamports)) {
        throw new Error(
            `Not enough compressed lamports. Expected ${lamports}, got ${inputLamports}`,
        );
    }

    const proof = await rpc.getValidityProof(
        userCompressedAccountsWithMerkleContext.map(x => bn(x.hash)),
    );

    const { blockhash } = await rpc.getLatestBlockhash();
    const ixs = await LightSystemProgram.decompress({
        payer: payer.publicKey,
        toAddress: recipient,
        outputStateTree: outputStateTree,
        inputCompressedAccounts: userCompressedAccountsWithMerkleContext,
        recentValidityProof: proof.compressedProof,
        recentInputStateRootIndices: proof.rootIndices,
        lamports,
    });

    const tx = buildAndSignTx(ixs, payer, blockhash, []);

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return txId;
}

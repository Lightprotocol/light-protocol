import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import { LightSystemProgram, sumUpLamports } from '../programs';
import { Rpc } from '../rpc';
import { buildAndSignTx, pickStateTreeInfo, sendAndConfirmTx } from '../utils';
import BN from 'bn.js';
import {
    CompressedAccountWithMerkleContext,
    StateTreeInfo,
    bn,
} from '../state';

/**
 * Decompress lamports into a solana account
 *
 * @param rpc                   RPC to use
 * @param payer                 Payer of the transaction and initialization fees
 * @param lamports              Amount of lamports to compress
 * @param toAddress             Address of the recipient compressed account
 * @param outputStateTreeInfo   Optional output state tree. Defaults to fetching
 *                              a current shared state tree.
 * @param confirmOptions        Options for confirming the transaction
 *
 * @return Transaction signature
 */
export async function decompress(
    rpc: Rpc,
    payer: Signer,
    lamports: number | BN,
    recipient: PublicKey,
    outputStateTreeInfo?: StateTreeInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    if (!outputStateTreeInfo) {
        const stateTreeInfo = await rpc.getCachedActiveStateTreeInfo();
        outputStateTreeInfo = pickStateTreeInfo(stateTreeInfo);
    }
    const userCompressedAccountsWithMerkleContext: CompressedAccountWithMerkleContext[] =
        (await rpc.getCompressedAccountsByOwner(payer.publicKey)).items;

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
    const ix = await LightSystemProgram.decompress({
        payer: payer.publicKey,
        toAddress: recipient,
        outputStateTreeInfo,
        inputCompressedAccounts: userCompressedAccountsWithMerkleContext,
        recentValidityProof: proof.compressedProof,
        recentInputStateRootIndices: proof.rootIndices,
        lamports,
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

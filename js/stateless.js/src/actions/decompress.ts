import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import { LightSystemProgram, sumUpLamports } from '../programs';
import { pickRandomStateTreeContext, Rpc } from '../rpc';
import { buildAndSignTx, sendAndConfirmTx } from '../utils';
import BN from 'bn.js';
import {
    CompressedAccountWithMerkleContext,
    StateTreeContext,
    bn,
} from '../state';

/**
 * Decompress lamports into a solana account
 *
 * @param rpc                       RPC to use
 * @param payer                     Payer of the transaction and initialization fees
 * @param lamports                  Amount of lamports to compress
 * @param toAddress                 Address of the recipient compressed account
 * @param outputStateTreeContext    Optional output state tree context.
 * @param confirmOptions            Options for confirming the transaction
 *
 * @return Transaction signature
 */
export async function decompress(
    rpc: Rpc,
    payer: Signer,
    lamports: number | BN,
    recipient: PublicKey,
    outputStateTreeContext?: StateTreeContext,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const userCompressedAccountsWithMerkleContext: CompressedAccountWithMerkleContext[] =
        (await rpc.getCompressedAccountsByOwner(payer.publicKey)).items;

    lamports = bn(lamports);

    if (!outputStateTreeContext) {
        const stateTreeInfo = await rpc.getCachedActiveStateTreeInfo();
        outputStateTreeContext = pickRandomStateTreeContext(stateTreeInfo);
    }

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
        outputStateTreeContext,
        inputCompressedAccounts: userCompressedAccountsWithMerkleContext,
        recentValidityProof: proof.compressedProof,
        recentInputStateRootIndices: proof.rootIndices,
        lamports,
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

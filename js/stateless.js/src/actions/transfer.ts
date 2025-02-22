import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';

import BN from 'bn.js';
import {
    LightSystemProgram,
    selectMinCompressedSolAccountsForTransfer,
} from '../programs';
import { pickRandomStateTreeContext, Rpc } from '../rpc';

import {
    bn,
    CompressedAccountWithMerkleContext,
    StateTreeContext,
    TreeType,
} from '../state';
import { buildAndSignTx, sendAndConfirmTx } from '../utils';
import { GetCompressedAccountsByOwnerConfig } from '../rpc-interface';
import { selectInputAccountsForTransfer } from '../utils/select-input-accounts';

/**
 * Transfer compressed lamports from one owner to another
 *
 * @param rpc                       Rpc to use
 * @param payer                     Payer of transaction fees
 * @param lamports                  Number of lamports to transfer
 * @param owner                     Owner of the compressed lamports
 * @param toAddress                 Destination address of the recipient
 * @param outputStateTreeContext    State tree context that the compressed lamports should be
 *                                  inserted into. Defaults to the default state tree context.
 * @param confirmOptions            Options for confirming the transaction
 *
 * @return Signature of the confirmed transaction
 */
export async function transfer(
    rpc: Rpc,
    payer: Signer,
    lamports: number | BN,
    owner: Signer,
    toAddress: PublicKey,
    outputStateTreeContext?: StateTreeContext,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    lamports = bn(lamports);

    if (!outputStateTreeContext) {
        const stateTreeInfo = await rpc.getCachedActiveStateTreeInfo();
        outputStateTreeContext = pickRandomStateTreeContext(
            stateTreeInfo,
            TreeType.BatchedState,
        );
    }

    const allAccounts = await rpc.getCompressedAccountsByOwner(owner.publicKey);

    const {
        selectedAccounts: potentialInputAccounts,
        inputLamports,
        discardedLamports,
    } = selectInputAccountsForTransfer(allAccounts.items, lamports);

    if (lamports.gt(inputLamports)) {
        throw new Error(
            `Insufficient balance for transfer. Required: ${lamports.toString()}, available: ${inputLamports.toString()}, unavailable: ${discardedLamports.toString()}`,
        );
    }

    const [inputAccounts] = selectMinCompressedSolAccountsForTransfer(
        potentialInputAccounts,
        lamports,
    );

    const proof = await rpc.getValidityProof(
        inputAccounts.map(account => bn(account.hash)),
    );

    const ix = await LightSystemProgram.transfer({
        payer: payer.publicKey,
        inputCompressedAccounts: inputAccounts,
        toAddress,
        lamports,
        recentInputStateRootIndices: proof.rootIndices,
        recentValidityProof: proof.compressedProof,
        outputStateTreeContext,
    });

    const { blockhash } = await rpc.getLatestBlockhash();
    const signedTx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 600_000 }), ix],
        payer,
        blockhash,
    );

    const txId = await sendAndConfirmTx(rpc, signedTx, confirmOptions);
    return txId;
}

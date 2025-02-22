import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import {
    LightSystemProgram,
    selectMinCompressedSolAccountsForTransfer,
} from '../programs';
import { pickStateTreeInfo } from '../utils/get-light-state-tree-info';
import { Rpc } from '../rpc';
import {
    buildAndSignTx,
    selectInputAccountsForTransfer,
    sendAndConfirmTx,
} from '../utils';
import { StateTreeInfo, TreeType, bn } from '../state';
import BN from 'bn.js';

/**
 * Decompress lamports into a solana account
 *
 * @param rpc                       RPC to use
 * @param payer                     Payer of the transaction and initialization fees
 * @param lamports                  Amount of lamports to compress
 * @param toAddress                 Address of the recipient compressed account
 * @param outputStateTreeInfo    Optional output state tree context.
 * @param confirmOptions            Options for confirming the transaction
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
    lamports = bn(lamports);
    const allAccounts = await rpc.getCompressedAccountsByOwner(payer.publicKey);

    const {
        selectedAccounts: maybeInputAccounts,
        inputLamports,
        discardedLamports,
    } = selectInputAccountsForTransfer(allAccounts.items, lamports);

    if (!outputStateTreeInfo) {
        const stateTreeInfo = await rpc.getCachedActiveStateTreeInfos();
        outputStateTreeInfo = pickStateTreeInfo(
            stateTreeInfo,
            TreeType.StateV2,
        );
    }

    if (lamports.gt(inputLamports)) {
        throw new Error(
            `Not enough compressed lamports. Expected ${lamports}, got ${inputLamports}, unavailable for this action: ${discardedLamports.toString()}`,
        );
    }

    const [inputAccounts] = selectMinCompressedSolAccountsForTransfer(
        maybeInputAccounts,
        lamports,
    );

    const proof = await rpc.getValidityProof(
        inputAccounts.map(x => bn(x.hash)),
    );

    const { blockhash } = await rpc.getLatestBlockhash();
    const ix = await LightSystemProgram.decompress({
        payer: payer.publicKey,
        toAddress: recipient,
        outputStateTreeInfo,
        inputCompressedAccounts: inputAccounts,
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

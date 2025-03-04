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
    sumUpLamports,
} from '../programs';
import { pickRandomStateTreeContext, Rpc } from '../rpc';
import {
    buildAndSignTx,
    selectInputAccountsForTransfer,
    sendAndConfirmTx,
    validateNumbers,
    validateNumbersForInclusionProof,
} from '../utils';
import BN from 'bn.js';
import {
    CompressedAccountWithMerkleContext,
    StateTreeContext,
    TreeType,
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
    lamports = bn(lamports);
    const allAccounts = await rpc.getCompressedAccountsByOwner(payer.publicKey);

    const {
        selectedAccounts: maybeInputAccounts,
        inputLamports,
        discardedLamports,
    } = selectInputAccountsForTransfer(allAccounts.items, lamports);

    if (!outputStateTreeContext) {
        const stateTreeInfo = await rpc.getCachedActiveStateTreeInfo();
        outputStateTreeContext = pickRandomStateTreeContext(
            stateTreeInfo,
            TreeType.BatchedState,
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
        outputStateTreeContext,
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

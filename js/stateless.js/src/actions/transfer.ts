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
import { Rpc } from '../rpc';
import { bn, CompressedAccountWithMerkleContext } from '../state';
import { buildAndSignTx, sendAndConfirmTx } from '../utils';
import { GetCompressedAccountsByOwnerConfig } from '../rpc-interface';

/**
 * Transfer compressed lamports from one owner to another
 *
 * @param rpc                   Rpc to use
 * @param payer                 Payer of transaction fees
 * @param lamports              Number of lamports to transfer
 * @param owner                 Owner of the compressed lamports
 * @param toAddress             Destination address of the recipient
 * @param confirmOptions        Options for confirming the transaction
 *
 * @return Signature of the confirmed transaction
 */
export async function transfer(
    rpc: Rpc,
    payer: Signer,
    lamports: number | BN,
    owner: Signer,
    toAddress: PublicKey,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    let accumulatedLamports = bn(0);
    const compressedAccounts: CompressedAccountWithMerkleContext[] = [];
    let cursor: string | undefined;
    const batchSize = 1000; // Maximum allowed by the API
    lamports = bn(lamports);

    while (accumulatedLamports.lt(lamports)) {
        const batchConfig: GetCompressedAccountsByOwnerConfig = {
            filters: undefined,
            dataSlice: undefined,
            cursor,
            limit: bn(batchSize),
        };

        const batch = await rpc.getCompressedAccountsByOwner(
            owner.publicKey,
            batchConfig,
        );

        for (const account of batch.items) {
            if (account.lamports.gt(bn(0))) {
                compressedAccounts.push(account);
                accumulatedLamports = accumulatedLamports.add(account.lamports);
            }
        }

        cursor = batch.cursor ?? undefined;
        if (batch.items.length < batchSize || accumulatedLamports.gte(lamports))
            break;
    }

    if (accumulatedLamports.lt(lamports)) {
        throw new Error(
            `Insufficient balance for transfer. Required: ${lamports.toString()}, available: ${accumulatedLamports.toString()}`,
        );
    }

    const [inputAccounts] = selectMinCompressedSolAccountsForTransfer(
        compressedAccounts,
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
    });

    const { blockhash } = await rpc.getLatestBlockhash();
    const signedTx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 350_000 }), ix],
        payer,
        blockhash,
    );

    const txId = await sendAndConfirmTx(rpc, signedTx, confirmOptions);
    return txId;
}

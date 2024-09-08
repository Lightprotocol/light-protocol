import {
    ComputeBudgetProgram,
    ConfirmOptions,
    DataSlice,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';

import { BN } from '@coral-xyz/anchor';
import {
    LightSystemProgram,
    selectMinCompressedSolAccountsForTransfer,
} from '../programs';
import { Rpc } from '../rpc';

import { bn, CompressedAccountWithMerkleContext } from '../state';
import { buildAndSignTx, sendAndConfirmTx } from '../utils';
import {
    GetCompressedAccountsByOwnerConfig,
    GetCompressedAccountsFilter,
} from '../rpc-interface';
import { string } from 'superstruct';

/**
 * Transfer compressed lamports from one owner to another
 *
 * @param rpc            Rpc to use
 * @param payer          Payer of transaction fees
 * @param lamports       Number of lamports to transfer
 * @param owner          Owner of the compressed lamports
 * @param toAddress      Destination address of the recipient
 * @param merkleTree     State tree account that the compressed lamports should be
 *                       inserted into. Defaults to the default state tree account.
 * @param confirmOptions Options for confirming the transaction
 * @param config         Configuration for fetching compressed accounts
 *
 *
 * @return Signature of the confirmed transaction
 */
export async function transfer(
    rpc: Rpc,
    payer: Signer,
    lamports: number | BN,
    owner: Signer,
    toAddress: PublicKey,
    /// TODO: allow multiple
    merkleTree?: PublicKey,
    confirmOptions?: ConfirmOptions,
    config?: Omit<GetCompressedAccountsByOwnerConfig, 'cursor' | 'limit'>,
): Promise<TransactionSignature> {
    let compressedAccounts: CompressedAccountWithMerkleContext[] = [];
    let cursor: string | null = null;
    const batchSize = 1000; // Maximum allowed by the API
    lamports = bn(lamports);

    do {
        const batchConfig: GetCompressedAccountsByOwnerConfig = {
            ...config,
            cursor,
            limit: new BN(batchSize),
        };
        const batch = await rpc.getCompressedAccountsByOwner(
            owner.publicKey,
            batchConfig,
        );
        const positiveBalanceAccounts = batch.items.filter(account =>
            account.lamports.gt(new BN(0)),
        );
        compressedAccounts = compressedAccounts.concat(positiveBalanceAccounts);
        cursor = batch.cursor;
        if (batch.items.length < batchSize) break;
    } while (cursor !== null);

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
        outputStateTrees: merkleTree,
    });

    const { blockhash } = await rpc.getLatestBlockhash();
    const signedTx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }), ix],
        payer,
        blockhash,
    );
    const txId = await sendAndConfirmTx(rpc, signedTx, confirmOptions);
    return txId;
}

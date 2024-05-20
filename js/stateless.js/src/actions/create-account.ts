import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';

import { LightSystemProgram } from '../programs';
import { Rpc } from '../rpc';
import {
    NewAddressParams,
    buildAndSignTx,
    deriveAddress,
    sendAndConfirmTx,
} from '../utils';
import { BN } from '@coral-xyz/anchor';
import { defaultTestStateTreeAccounts } from '../constants';
import { TestRpc } from '../test-helpers';
import { bn } from '../state';

/**
 * Compress lamports to a solana address
 *
 * @param rpc             RPC to use
 * @param payer           Payer of the transaction and initialization fees
 * @param seed            Seed to derive the new account address
 * @param programId       Owner of the new account
 * @param addressTree     Optional address tree. Defaults to a current shared address tree.
 * @param addressQueue    Optional address queue. Defaults to a current shared address queue.
 * @param outputStateTree Optional output state tree. Defaults to a current shared state tree.
 * @param confirmOptions  Options for confirming the transaction
 *
 * @return Transaction signature
 */
/// TODO: test-rpc -> rpc
export async function createAccount(
    rpc: TestRpc,
    payer: Signer,
    seed: Uint8Array,
    programId: PublicKey,
    addressTree?: PublicKey,
    addressQueue?: PublicKey,
    outputStateTree?: PublicKey,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const { blockhash } = await rpc.getLatestBlockhash();

    addressTree = addressTree ?? defaultTestStateTreeAccounts().addressTree;
    addressQueue = addressQueue ?? defaultTestStateTreeAccounts().addressQueue;

    /// TODO: enforce program-derived
    const address = await deriveAddress(seed, addressTree);

    /// TODO: pass trees
    const proof = await rpc.getNewAddressValidityProof(address);

    const params: NewAddressParams = {
        seed: seed,
        addressMerkleTreeRootIndex: proof.rootIndices[0],
        addressMerkleTreePubkey: proof.merkleTrees[0],
        addressQueuePubkey: proof.nullifierQueues[0],
    };

    const ix = await LightSystemProgram.createAccount({
        payer: payer.publicKey,
        newAddressParams: params,
        newAddress: Array.from(address.toBytes()),
        recentValidityProof: proof.compressedProof,
        programId,
        outputStateTree: outputStateTree
            ? outputStateTree
            : defaultTestStateTreeAccounts().merkleTree, // TODO: should fetch the current shared state tree
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

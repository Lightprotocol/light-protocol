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
import { Rpc } from '../rpc';
import {
    NewAddressParams,
    buildAndSignTx,
    deriveAddress,
    sendAndConfirmTx,
} from '../utils';
import { defaultTestStateTreeAccounts } from '../constants';
import { bn } from '../state';
import { BN } from '@coral-xyz/anchor';

/**
 * Create compressed account with address
 *
 * @param rpc               RPC to use
 * @param payer             Payer of the transaction and initialization fees
 * @param seed              Seed to derive the new account address
 * @param programId         Owner of the new account
 * @param addressTree       Optional address tree. Defaults to a current shared
 *                          address tree.
 * @param addressQueue      Optional address queue. Defaults to a current shared
 *                          address queue.
 * @param outputStateTree   Optional output state tree. Defaults to a current
 *                          shared state tree.
 * @param confirmOptions    Options for confirming the transaction
 *
 * @return                  Transaction signature
 */
export async function createAccount(
    rpc: Rpc,
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
    const proof = await rpc.getValidityProof(undefined, [
        bn(address.toBytes()),
    ]);

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
        outputStateTree,
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

/**
 * Create compressed account with address and lamports
 *
 * @param rpc               RPC to use
 * @param payer             Payer of the transaction and initialization fees
 * @param seed              Seed to derive the new account address
 * @param lamports          Number of compressed lamports to initialize the
 *                          account with
 * @param programId         Owner of the new account
 * @param addressTree       Optional address tree. Defaults to a current shared
 *                          address tree.
 * @param addressQueue      Optional address queue. Defaults to a current shared
 *                          address queue.
 * @param outputStateTree   Optional output state tree. Defaults to a current
 *                          shared state tree.
 * @param confirmOptions    Options for confirming the transaction
 *
 * @return                  Transaction signature
 */
// TODO: add support for payer != user owner
export async function createAccountWithLamports(
    rpc: Rpc,
    payer: Signer,
    seed: Uint8Array,
    lamports: number | BN,
    programId: PublicKey,
    addressTree?: PublicKey,
    addressQueue?: PublicKey,
    outputStateTree?: PublicKey,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    lamports = bn(lamports);

    const compressedAccounts = await rpc.getCompressedAccountsByOwner(
        payer.publicKey,
    );

    const [inputAccounts] = selectMinCompressedSolAccountsForTransfer(
        compressedAccounts,
        lamports,
    );

    const { blockhash } = await rpc.getLatestBlockhash();

    addressTree = addressTree ?? defaultTestStateTreeAccounts().addressTree;
    addressQueue = addressQueue ?? defaultTestStateTreeAccounts().addressQueue;

    /// TODO: enforce program-derived
    const address = await deriveAddress(seed, addressTree);

    const proof = await rpc.getValidityProof(
        inputAccounts.map(account => bn(account.hash)),
        [bn(address.toBytes())],
    );

    /// TODO(crank): Adapt before supporting addresses in rpc / cranked address trees.
    /// Currently expects address roots to be consistent with one another and
    /// static. See test-rpc.ts for more details.
    const params: NewAddressParams = {
        seed: seed,
        addressMerkleTreeRootIndex:
            proof.rootIndices[proof.rootIndices.length - 1],
        addressMerkleTreePubkey:
            proof.merkleTrees[proof.merkleTrees.length - 1],
        addressQueuePubkey:
            proof.nullifierQueues[proof.nullifierQueues.length - 1],
    };

    const ix = await LightSystemProgram.createAccount({
        payer: payer.publicKey,
        newAddressParams: params,
        newAddress: Array.from(address.toBytes()),
        recentValidityProof: proof.compressedProof,
        inputCompressedAccounts: inputAccounts,
        inputStateRootIndices: proof.rootIndices,
        programId,
        outputStateTree,
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

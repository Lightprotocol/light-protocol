import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import {
    LightSystemProgram,
    selectMinCompressedSolAccountsForPdaCreation,
} from '../programs';
import { pickStateTreeInfo } from '../utils/get-light-state-tree-info';
import { Rpc } from '../rpc';
import {
    NewAddressParams,
    buildAndSignTx,
    deriveAddress,
    deriveAddressSeed,
    sendAndConfirmTx,
} from '../utils';
import { defaultTestStateTreeAccounts } from '../constants';
import { bn, StateTreeInfo, TreeType } from '../state';
import BN from 'bn.js';

/**
 * Create compressed account with address
 *
 * @param rpc               RPC to use
 * @param payer             Payer of the transaction and initialization fees
 * @param seeds             Seeds to derive the new account address
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
    seeds: Uint8Array[],
    programId: PublicKey,
    addressTree?: PublicKey,
    addressQueue?: PublicKey,
    outputStateTreeInfo?: StateTreeInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const { blockhash } = await rpc.getLatestBlockhash();

    // TODO: add V2 support.
    addressTree = addressTree ?? defaultTestStateTreeAccounts().addressTree;
    addressQueue = addressQueue ?? defaultTestStateTreeAccounts().addressQueue;

    const seed = deriveAddressSeed(seeds, programId);
    const address = deriveAddress(seed, addressTree);

    if (!outputStateTreeInfo) {
        const stateTreeInfo = await rpc.getCachedActiveStateTreeInfos();
        outputStateTreeInfo = pickStateTreeInfo(
            stateTreeInfo,
            TreeType.StateV2,
        );
    }

    const proof = await rpc.getValidityProofV0(undefined, [
        {
            address: bn(address.toBytes()),
            tree: addressTree,
            queue: addressQueue,
        },
    ]);

    const params: NewAddressParams = {
        seed: seed,
        addressMerkleTreeRootIndex: proof.rootIndices[0],
        addressMerkleTreePubkey: proof.merkleTrees[0],
        addressQueuePubkey: proof.queues[0],
    };

    const ix = await LightSystemProgram.createAccount({
        payer: payer.publicKey,
        newAddressParams: params,
        newAddress: Array.from(address.toBytes()),
        recentValidityProof: proof.compressedProof!,
        programId,
        outputStateTreeInfo,
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

/**
 * Create compressed account with address and lamports
 *
 * @param rpc                       RPC to use
 * @param payer                     Payer of the transaction and initialization fees
 * @param seeds                     Seeds to derive the new account address
 * @param lamports                  Number of compressed lamports to initialize the
 *                                  account with
 * @param programId                 Owner of the new account
 * @param addressTree               Optional address tree. Defaults to a current shared
 *                                  address tree.
 * @param addressQueue              Optional address queue. Defaults to a current shared
 *                                  address queue.
 * @param outputStateTreeInfo    Optional output state tree context.
 * @param confirmOptions            Options for confirming the transaction
 *
 * @return  Transaction signature
 */
export async function createAccountWithLamports(
    rpc: Rpc,
    payer: Signer,
    seeds: Uint8Array[],
    lamports: number | BN,
    programId: PublicKey,
    addressTree?: PublicKey,
    addressQueue?: PublicKey,
    outputStateTreeInfo?: StateTreeInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    lamports = bn(lamports);

    const compressedAccounts = await rpc.getCompressedAccountsByOwner(
        payer.publicKey,
    );

    const [inputAccounts] = selectMinCompressedSolAccountsForPdaCreation(
        compressedAccounts.items,
        lamports,
    );
    if (!outputStateTreeInfo) {
        const stateTreeInfo = await rpc.getCachedActiveStateTreeInfos();
        outputStateTreeInfo = pickStateTreeInfo(
            stateTreeInfo,
            TreeType.StateV2,
        );
    }

    const { blockhash } = await rpc.getLatestBlockhash();

    addressTree = addressTree ?? defaultTestStateTreeAccounts().addressTree;
    addressQueue = addressQueue ?? defaultTestStateTreeAccounts().addressQueue;

    const seed = deriveAddressSeed(seeds, programId);
    const address = deriveAddress(seed, addressTree);

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
        addressQueuePubkey: proof.queues[proof.queues.length - 1],
    };

    const ix = await LightSystemProgram.createAccount({
        payer: payer.publicKey,
        newAddressParams: params,
        newAddress: Array.from(address.toBytes()),
        recentValidityProof: proof.compressedProof!,
        inputCompressedAccounts: inputAccounts,
        inputStateRootIndices: proof.rootIndices,
        programId,
        outputStateTreeInfo,
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

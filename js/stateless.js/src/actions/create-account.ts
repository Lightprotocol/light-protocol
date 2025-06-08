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
    deriveAddressSeed,
    selectStateTreeInfo,
    sendAndConfirmTx,
} from '../utils';
import { getDefaultAddressTreeInfo } from '../constants';
import { AddressTreeInfo, bn, TreeInfo } from '../state';
import BN from 'bn.js';

/**
 * Create compressed account with address
 *
 * @param rpc                   RPC to use
 * @param payer                 Payer of the transaction and initialization fees
 * @param seeds                 Seeds to derive the new account address
 * @param programId             Owner of the new account
 * @param addressTreeInfo       Optional address tree info. Defaults to a current
 *                              shared address tree.
 * @param outputStateTreeInfo   Optional output state tree. Defaults to fetching
 *                              a current shared state tree.
 * @param confirmOptions        Options for confirming the transaction
 *
 * @return Transaction signature
 */
export async function createAccount(
    rpc: Rpc,
    payer: Signer,
    seeds: Uint8Array[],
    programId: PublicKey,
    addressTreeInfo?: AddressTreeInfo,
    outputStateTreeInfo?: TreeInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const { blockhash } = await rpc.getLatestBlockhash();
    const { tree, queue } = addressTreeInfo ?? getDefaultAddressTreeInfo();

    const seed = deriveAddressSeed(seeds, programId);
    const address = deriveAddress(seed, tree);

    if (!outputStateTreeInfo) {
        const stateTreeInfo = await rpc.getStateTreeInfos();
        outputStateTreeInfo = selectStateTreeInfo(stateTreeInfo);
    }

    const proof = await rpc.getValidityProofV0(undefined, [
        {
            address: bn(address.toBytes()),
            tree,
            queue,
        },
    ]);

    const params: NewAddressParams = {
        seed: seed,
        addressMerkleTreeRootIndex: proof.rootIndices[0],
        addressMerkleTreePubkey: proof.treeInfos[0].tree,
        addressQueuePubkey: proof.treeInfos[0].queue,
    };

    const ix = await LightSystemProgram.createAccount({
        payer: payer.publicKey,
        newAddressParams: params,
        newAddress: Array.from(address.toBytes()),
        recentValidityProof: proof.compressedProof,
        programId,
        outputStateTreeInfo,
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
 * @param rpc                   RPC to use
 * @param payer                 Payer of the transaction and initialization fees
 * @param seeds                 Seeds to derive the new account address
 * @param lamports              Number of compressed lamports to initialize the
 *                              account with
 * @param programId             Owner of the new account
 * @param addressTreeInfo       Optional address tree info. Defaults to a
 *                              current shared address tree.
 * @param outputStateTreeInfo   Optional output state tree. Defaults to a
 *                              current shared state tree.
 * @param confirmOptions        Options for confirming the transaction
 *
 * @return Transaction signature
 */
export async function createAccountWithLamports(
    rpc: Rpc,
    payer: Signer,
    seeds: Uint8Array[],
    lamports: number | BN,
    programId: PublicKey,
    addressTreeInfo?: AddressTreeInfo,
    outputStateTreeInfo?: TreeInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    lamports = bn(lamports);

    const compressedAccounts = await rpc.getCompressedAccountsByOwner(
        payer.publicKey,
    );

    const [inputAccounts] = selectMinCompressedSolAccountsForTransfer(
        compressedAccounts.items,
        lamports,
    );

    const { blockhash } = await rpc.getLatestBlockhash();

    const { tree } = addressTreeInfo ?? getDefaultAddressTreeInfo();

    const seed = deriveAddressSeed(seeds, programId);
    const address = deriveAddress(seed, tree);

    const proof = await rpc.getValidityProof(
        inputAccounts.map(account => account.hash),
        [bn(address.toBytes())],
    );

    const params: NewAddressParams = {
        seed: seed,
        addressMerkleTreeRootIndex:
            proof.rootIndices[proof.rootIndices.length - 1],
        addressMerkleTreePubkey:
            proof.treeInfos[proof.treeInfos.length - 1].tree,
        addressQueuePubkey: proof.treeInfos[proof.treeInfos.length - 1].queue,
    };

    const ix = await LightSystemProgram.createAccount({
        payer: payer.publicKey,
        newAddressParams: params,
        newAddress: Array.from(address.toBytes()),
        recentValidityProof: proof.compressedProof,
        inputCompressedAccounts: inputAccounts,
        inputStateRootIndices: proof.rootIndices,
        outputStateTreeInfo,
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

import { AccountMeta, PublicKey } from '@solana/web3.js';
import {
    AccountProofInput,
    CompressedAccountLegacy,
    NewAddressProofInput,
    OutputCompressedAccountWithPackedContext,
    PackedCompressedAccountWithMerkleContext,
    TreeInfo,
    TreeType,
} from '../../state';
import {
    CompressedAccountWithMerkleContextLegacy,
    PackedAddressTreeInfo,
    PackedStateTreeInfo,
} from '../../state/compressed-account';
import { featureFlags } from '../../constants';

/**
 * @internal Finds the index of a PublicKey in an array, or adds it if not
 * present
 * */
export function getIndexOrAdd(
    accountsArray: PublicKey[],
    key: PublicKey,
): number {
    const index = accountsArray.findIndex(existingKey =>
        existingKey.equals(key),
    );
    if (index === -1) {
        accountsArray.push(key);
        return accountsArray.length - 1;
    }
    return index;
}

/**
 * @internal
 * Pads output state trees with the 0th state tree of the input state.
 *
 * @param outputStateMerkleTrees                    Optional output state trees
 *                                                  to be inserted into the
 *                                                  output state. Defaults to
 *                                                  the 0th state tree of the
 *                                                  input state. Gets padded to
 *                                                  the length of
 *                                                  outputCompressedAccounts.
 * @param numberOfOutputCompressedAccounts          The number of output
 *                                                  compressed accounts.
 *
 * @returns Padded output state trees.
 */
export function padOutputStateMerkleTrees(
    outputStateMerkleTrees: PublicKey,
    numberOfOutputCompressedAccounts: number,
): PublicKey[] {
    if (numberOfOutputCompressedAccounts <= 0) {
        return [];
    }

    return new Array(numberOfOutputCompressedAccounts).fill(
        outputStateMerkleTrees,
    );
}

export function toAccountMetas(remainingAccounts: PublicKey[]): AccountMeta[] {
    return remainingAccounts.map(
        (account): AccountMeta => ({
            pubkey: account,
            isWritable: true,
            isSigner: false,
        }),
    );
}

export interface PackedStateTreeInfos {
    packedTreeInfos: PackedStateTreeInfo[];
    outputTreeIndex: number;
}

export interface PackedTreeInfos {
    stateTrees?: PackedStateTreeInfos;
    addressTrees: PackedAddressTreeInfo[];
}

const INVALID_TREE_INDEX = -1;
/**
 * Packs TreeInfos. Replaces PublicKey with index pointer to remaining accounts.
 *
 * Only use for MUT, CLOSE, NEW_ADDRESSES. For INIT, pass
 * {@link newAddressParamsPacked} and `outputStateTreeIndex` to your program
 * instead.
 *
 *
 * @param remainingAccounts                 Optional existing array of accounts
 *                                          to append to.
 * @param accountProofInputs                Account proof inputs.
 * @param newAddressProofInputs             New address proof inputs.
 *
 * @returns Remaining accounts, packed state and address tree infos, state tree
 * output index and address tree infos.
 */
export function packTreeInfos(
    remainingAccounts: PublicKey[],
    accountProofInputs: AccountProofInput[],
    newAddressProofInputs: NewAddressProofInput[],
): PackedTreeInfos {
    const _remainingAccounts = remainingAccounts.slice();

    const stateTreeInfos: PackedStateTreeInfo[] = [];
    const addressTreeInfos: PackedAddressTreeInfo[] = [];
    let outputTreeIndex: number = INVALID_TREE_INDEX;

    // Early exit.
    if (accountProofInputs.length === 0 && newAddressProofInputs.length === 0) {
        return {
            stateTrees: undefined,
            addressTrees: addressTreeInfos,
        };
    }

    // input
    accountProofInputs.forEach((account, index) => {
        const merkleTreePubkeyIndex = getIndexOrAdd(
            _remainingAccounts,
            account.treeInfo.tree,
        );

        const queuePubkeyIndex = getIndexOrAdd(
            _remainingAccounts,
            account.treeInfo.queue,
        );

        stateTreeInfos.push({
            rootIndex: account.rootIndex,
            merkleTreePubkeyIndex,
            queuePubkeyIndex,
            leafIndex: account.leafIndex,
            proveByIndex: account.proveByIndex,
        });
    });

    // output
    if (stateTreeInfos.length > 0) {
        // Use next tree if available, otherwise fall back to current tree.
        // `nextTreeInfo` always takes precedence.
        const activeTreeInfo =
            accountProofInputs[0].treeInfo.nextTreeInfo ||
            accountProofInputs[0].treeInfo;
        let activeTreeOrQueue = activeTreeInfo.tree;

        if (activeTreeInfo.treeType === TreeType.StateV2) {
            if (featureFlags.isV2()) {
                activeTreeOrQueue = activeTreeInfo.queue;
            } else throw new Error('V2 trees are not supported yet');
        }
        outputTreeIndex = getIndexOrAdd(_remainingAccounts, activeTreeOrQueue);
    }

    // new addresses
    newAddressProofInputs.forEach((account, index) => {
        const addressMerkleTreePubkeyIndex = getIndexOrAdd(
            _remainingAccounts,
            account.treeInfo.tree,
        );
        const addressQueuePubkeyIndex = getIndexOrAdd(
            _remainingAccounts,
            account.treeInfo.queue,
        );

        addressTreeInfos.push({
            rootIndex: account.rootIndex,
            addressMerkleTreePubkeyIndex,
            addressQueuePubkeyIndex,
        });
    });

    return {
        stateTrees:
            stateTreeInfos.length > 0
                ? {
                      packedTreeInfos: stateTreeInfos,
                      outputTreeIndex,
                  }
                : undefined,
        addressTrees: addressTreeInfos,
    };
}

/**
 * Packs Compressed Accounts.
 *
 * Replaces PublicKey with index pointer to remaining accounts.
 *
 *
 * @param inputCompressedAccounts           Ix input state to be consumed
 * @param inputStateRootIndices             The recent state root indices of the
 *                                          input state. The expiry is tied to
 *                                          the proof.
 * @param outputCompressedAccounts          Ix output state to be created
 * @param outputStateTreeInfo               The output state tree info. Gets
 *                                          padded to the length of
 *                                          outputCompressedAccounts.
 *
 * @param remainingAccounts                 Optional existing array of accounts
 *                                          to append to.
 **/
export function packCompressedAccounts(
    inputCompressedAccounts: CompressedAccountWithMerkleContextLegacy[],
    inputStateRootIndices: number[],
    outputCompressedAccounts: CompressedAccountLegacy[],
    outputStateTreeInfo?: TreeInfo,
    remainingAccounts: PublicKey[] = [],
): {
    packedInputCompressedAccounts: PackedCompressedAccountWithMerkleContext[];
    packedOutputCompressedAccounts: OutputCompressedAccountWithPackedContext[];
    remainingAccounts: PublicKey[];
} {
    const _remainingAccounts = remainingAccounts.slice();

    const packedInputCompressedAccounts: PackedCompressedAccountWithMerkleContext[] =
        [];

    const packedOutputCompressedAccounts: OutputCompressedAccountWithPackedContext[] =
        [];

    /// input
    inputCompressedAccounts.forEach((account, index) => {
        const merkleTreePubkeyIndex = getIndexOrAdd(
            _remainingAccounts,
            account.treeInfo.tree,
        );

        const queuePubkeyIndex = getIndexOrAdd(
            _remainingAccounts,
            account.treeInfo.queue,
        );

        packedInputCompressedAccounts.push({
            compressedAccount: {
                owner: account.owner,
                lamports: account.lamports,
                address: account.address,
                data: account.data,
            },
            merkleContext: {
                merkleTreePubkeyIndex,
                queuePubkeyIndex,
                leafIndex: account.leafIndex,
                proveByIndex: account.proveByIndex,
            },
            rootIndex: inputStateRootIndices[index],
            readOnly: false,
        });
    });
    if (inputCompressedAccounts.length > 0 && outputStateTreeInfo) {
        throw new Error(
            'Cannot specify both input accounts and outputStateTreeInfo',
        );
    }

    let treeInfo: TreeInfo;
    if (inputCompressedAccounts.length > 0) {
        treeInfo = inputCompressedAccounts[0].treeInfo;
    } else if (outputStateTreeInfo) {
        treeInfo = outputStateTreeInfo;
    } else {
        throw new Error(
            'Neither input accounts nor outputStateTreeInfo are available',
        );
    }

    // Use next tree if available, otherwise fall back to current tree.
    // `nextTreeInfo` always takes precedence.
    const activeTreeInfo = treeInfo.nextTreeInfo || treeInfo;
    let activeTreeOrQueue = activeTreeInfo.tree;

    if (activeTreeInfo.treeType === TreeType.StateV2) {
        if (featureFlags.isV2()) {
            activeTreeOrQueue = activeTreeInfo.queue;
        } else throw new Error('V2 trees are not supported yet');
    }
    /// output
    const paddedOutputStateMerkleTrees = padOutputStateMerkleTrees(
        activeTreeOrQueue,
        outputCompressedAccounts.length,
    );

    outputCompressedAccounts.forEach((account, index) => {
        const merkleTreePubkeyIndex = getIndexOrAdd(
            _remainingAccounts,
            paddedOutputStateMerkleTrees[index],
        );
        packedOutputCompressedAccounts.push({
            compressedAccount: {
                owner: account.owner,
                lamports: account.lamports,
                address: account.address,
                data: account.data,
            },
            merkleTreeIndex: merkleTreePubkeyIndex,
        });
    });

    return {
        packedInputCompressedAccounts,
        packedOutputCompressedAccounts,
        remainingAccounts: _remainingAccounts,
    };
}

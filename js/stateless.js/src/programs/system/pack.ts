import { AccountMeta, PublicKey } from '@solana/web3.js';
import {
    CompressedAccount,
    OutputCompressedAccountWithPackedContext,
    PackedCompressedAccountWithMerkleContext,
    StateTreeInfo,
    TreeType,
} from '../../state';
import { CompressedAccountWithMerkleContext } from '../../state/compressed-account';
import { toArray } from '../../utils/conversion';

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
    inputCompressedAccounts: CompressedAccountWithMerkleContext[],
    inputStateRootIndices: number[],
    outputCompressedAccounts: CompressedAccount[],
    outputStateTreeInfo?: StateTreeInfo,
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

        const nullifierQueuePubkeyIndex = getIndexOrAdd(
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
                nullifierQueuePubkeyIndex,
                leafIndex: account.leafIndex,
                queueIndex: null,
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

    let treeInfo: StateTreeInfo;
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
    const activeTreeOrQueue = activeTreeInfo.tree;
    // V2 trees are not yet supported
    if (activeTreeInfo.treeType === TreeType.StateV2) {
        throw new Error('V2 trees are not supported yet');
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

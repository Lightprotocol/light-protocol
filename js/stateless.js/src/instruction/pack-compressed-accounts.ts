import { AccountMeta, PublicKey } from '@solana/web3.js';
import {
    CompressedAccount,
    OutputCompressedAccountWithPackedContext,
    PackedCompressedAccountWithMerkleContext,
    StateTreeInfo,
    TreeType,
} from '../state';
import { CompressedAccountWithMerkleContext } from '../state/compressed-account';
import { toArray } from '../utils/conversion';

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
 * @param outputStateMerkleTrees            Optional output state trees to be
 *                                          inserted into the output state.
 *                                          Defaults to the 0th state tree of
 *                                          the input state. Gets padded to the
 *                                          length of outputCompressedAccounts.
 * @param numberOfOutputCompressedAccounts  The number of output compressed
 *                                          accounts.
 * @param inputCompressedAccountsWithMerkleContext The input compressed accounts
 *                                          with merkle context.
 *
 * @returns Padded output state trees.
 */
export function padOutputStateMerkleTrees(
    outputStateMerkleTrees: PublicKey[] | PublicKey | undefined,
    numberOfOutputCompressedAccounts: number,
    inputCompressedAccountsWithMerkleContext: CompressedAccountWithMerkleContext[],
): PublicKey[] {
    if (numberOfOutputCompressedAccounts <= 0) {
        return [];
    }

    /// Default: use the 0th state tree of input state for all output accounts
    if (outputStateMerkleTrees === undefined) {
        if (inputCompressedAccountsWithMerkleContext.length === 0) {
            throw new Error(
                'No input compressed accounts nor output state trees provided. Please pass in at least one of the following: outputStateMerkleTree or inputCompressedAccount',
            );
        }
        return new Array(numberOfOutputCompressedAccounts).fill(
            inputCompressedAccountsWithMerkleContext[0].merkleTree,
        );
        /// Align the number of output state trees with the number of output
        /// accounts, and fill up with 0th output state tree
    } else {
        /// Into array
        const treesArray = toArray(outputStateMerkleTrees);
        if (treesArray.length >= numberOfOutputCompressedAccounts) {
            return treesArray.slice(0, numberOfOutputCompressedAccounts);
        } else {
            return treesArray.concat(
                new Array(
                    numberOfOutputCompressedAccounts - treesArray.length,
                ).fill(treesArray[0]),
            );
        }
    }
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
 * @param outputStateTreeInfo            Context of state tree to be inserted
 *                                          into the output state. Gets padded
 *                                          to the length of
 *                                          outputCompressedAccounts.
 *
 * @param remainingAccounts                 Optional existing array of accounts
 *                                          to append to.
 **/
export function packCompressedAccounts(
    inputCompressedAccounts: CompressedAccountWithMerkleContext[],
    inputStateRootIndices: number[],
    outputCompressedAccounts: CompressedAccount[],
    outputStateTreeInfo: StateTreeInfo,
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
            account.merkleTree,
        );

        const queuePubkeyIndex = getIndexOrAdd(
            _remainingAccounts,
            account.queue,
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

    // internal. v2 trees require the output queue account instead of directly
    // appending to the merkle tree.
    const outputTreeOrQueue =
        outputStateTreeInfo.treeType === TreeType.StateV2
            ? outputStateTreeInfo.queue!
            : outputStateTreeInfo.tree;

    /// output
    const paddedOutputStateMerkleTrees = padOutputStateMerkleTrees(
        outputTreeOrQueue,
        outputCompressedAccounts.length,
        inputCompressedAccounts,
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

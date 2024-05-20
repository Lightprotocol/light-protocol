import { AccountMeta, PublicKey } from '@solana/web3.js';
import {
    CompressedAccount,
    OutputCompressedAccountWithPackedContext,
    PackedCompressedAccountWithMerkleContext,
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

/** @internal */
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
                'inputCompressedAccountsWithMerkleContext cannot be empty when outputStateMerkleTrees is undefined',
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

// TODO: include owner and lamports in packing.
/**
 * Packs Compressed Accounts.
 *
 * Replaces PublicKey with index pointer to remaining accounts.
 *
 * @param inputCompressedAccounts           Ix input state to be consumed
 * @param inputStateRootIndices             The recent state root indices of the
 *                                          input state. The expiry is tied to
 *                                          the proof.
 * @param outputCompressedAccounts          Ix output state to be created
 * @param outputStateMerkleTrees            Optional output state trees to be
 *                                          inserted into the output state.
 *                                          Defaults to the 0th state tree of
 *                                          the input state. Gets padded to the
 *                                          length of outputCompressedAccounts.
 *
 * @param remainingAccounts                 Optional existing array of accounts
 *                                          to append to.
 **/
export function packCompressedAccounts(
    inputCompressedAccounts: CompressedAccountWithMerkleContext[],
    inputStateRootIndices: number[],
    outputCompressedAccounts: CompressedAccount[],
    outputStateMerkleTrees?: PublicKey[] | PublicKey,
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

        const nullifierQueuePubkeyIndex = getIndexOrAdd(
            _remainingAccounts,
            account.nullifierQueue,
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
            },
            rootIndex: inputStateRootIndices[index],
        });
    });

    /// output
    const paddedOutputStateMerkleTrees = padOutputStateMerkleTrees(
        outputStateMerkleTrees,
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

//@ts-ignore
if (import.meta.vitest) {
    //@ts-ignore
    const { describe, it, expect } = import.meta.vitest;
    // Inline unit tests for padOutputStateMerkleTrees function
    describe('padOutputStateMerkleTrees', () => {
        const treeA: any = PublicKey.unique();
        const treeB: any = PublicKey.unique();
        const treeC: any = PublicKey.unique();

        const accA: any = { merkleTree: treeA };
        const accB: any = { merkleTree: treeB };
        const accC: any = { merkleTree: treeC };

        it('should use the 0th state tree of input state if no output state trees are provided', () => {
            const result = padOutputStateMerkleTrees(undefined, 3, [
                accA,
                accB,
            ]);
            expect(result).toEqual([treeA, treeA, treeA]);
        });

        it('should fill up with the first state tree if provided trees are less than required', () => {
            const result = padOutputStateMerkleTrees([treeA, treeB], 5, []);
            expect(result).toEqual([treeA, treeB, treeA, treeA, treeA]);
        });

        it('should remove extra trees if the number of output state trees is greater than the number of output accounts', () => {
            const result = padOutputStateMerkleTrees(
                [treeA, treeB, treeC],
                2,
                [],
            );
            expect(result).toEqual([treeA, treeB]);
        });

        it('should return the same outputStateMerkleTrees if its length equals the number of output compressed accounts', () => {
            const result = padOutputStateMerkleTrees(
                [treeA, treeB, treeC],
                3,
                [],
            );
            expect(result).toEqual([treeA, treeB, treeC]);
        });
    });
}

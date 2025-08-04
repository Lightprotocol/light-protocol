import { AccountMeta, PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import {
    AccountProofInput,
    CompressedAccountLegacy,
    NewAddressProofInput,
    OutputCompressedAccountWithPackedContext,
    PackedCompressedAccountWithMerkleContext,
    TreeInfo,
    TreeType,
    ValidityProof,
} from '../../state';
import { ValidityProofWithContext } from '../../rpc-interface';
import {
    CompressedAccountWithMerkleContextLegacy,
    PackedAddressTreeInfo,
    PackedStateTreeInfo,
} from '../../state/compressed-account';
import { featureFlags } from '../../constants';
import { PackedAccounts } from '../../utils';

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

const INVALID_TREE_INDEX = -1;

/**
 * @deprecated Use {@link packTreeInfos} instead.
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
export function packTreeInfosWithPubkeys(
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
            stateTrees: null,
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
                : null,
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

/**
 * Root index for state tree proofs.
 */
export type RootIndex = {
    proofByIndex: boolean;
    rootIndex: number;
};

/**
 * Creates a RootIndex for proving by merkle proof.
 */
export function createRootIndex(rootIndex: number): RootIndex {
    return {
        proofByIndex: false,
        rootIndex,
    };
}

/**
 * Creates a RootIndex for proving by leaf index.
 */
export function createRootIndexByIndex(): RootIndex {
    return {
        proofByIndex: true,
        rootIndex: 0,
    };
}

/**
 * Account proof inputs for state tree accounts.
 */
export type AccountProofInputs = {
    hash: Uint8Array;
    root: Uint8Array;
    rootIndex: RootIndex;
    leafIndex: number;
    treeInfo: TreeInfo;
};

/**
 * Address proof inputs for address tree accounts.
 */
export type AddressProofInputs = {
    address: Uint8Array;
    root: Uint8Array;
    rootIndex: number;
    treeInfo: TreeInfo;
};

/**
 * Validity proof with context structure that matches Rust implementation.
 */
export type ValidityProofWithContextV2 = {
    proof: ValidityProof | null;
    accounts: AccountProofInputs[];
    addresses: AddressProofInputs[];
};

/**
 * Packed state tree infos.
 */
export type PackedStateTreeInfos = {
    packedTreeInfos: PackedStateTreeInfo[];
    outputTreeIndex: number;
};

/**
 * Packed tree infos containing both state and address trees.
 */
export type PackedTreeInfos = {
    stateTrees: PackedStateTreeInfos | null;
    addressTrees: PackedAddressTreeInfo[];
};

/**
 * Packs the output tree index based on tree type.
 * For StateV1, returns the index of the tree account.
 * For StateV2, returns the index of the queue account.
 */
function packOutputTreeIndex(
    treeInfo: TreeInfo,
    packedAccounts: PackedAccounts,
): number {
    switch (treeInfo.treeType) {
        case TreeType.StateV1:
            return packedAccounts.insertOrGet(treeInfo.tree);
        case TreeType.StateV2:
            return packedAccounts.insertOrGet(treeInfo.queue);
        default:
            throw new Error('Invalid tree type for packing output tree index');
    }
}

/**
 * Converts ValidityProofWithContext to ValidityProofWithContextV2 format.
 * Infers the split between state and address accounts based on tree types.
 */
function convertValidityProofToV2(
    validityProof: ValidityProofWithContext,
): ValidityProofWithContextV2 {
    const accounts: AccountProofInputs[] = [];
    const addresses: AddressProofInputs[] = [];

    for (let i = 0; i < validityProof.treeInfos.length; i++) {
        const treeInfo = validityProof.treeInfos[i];

        if (
            treeInfo.treeType === TreeType.StateV1 ||
            treeInfo.treeType === TreeType.StateV2
        ) {
            // State tree account
            accounts.push({
                hash: new Uint8Array(validityProof.leaves[i].toArray('le', 32)),
                root: new Uint8Array(validityProof.roots[i].toArray('le', 32)),
                rootIndex: {
                    proofByIndex: validityProof.proveByIndices[i],
                    rootIndex: validityProof.rootIndices[i],
                },
                leafIndex: validityProof.leafIndices[i],
                treeInfo,
            });
        } else {
            // Address tree account
            addresses.push({
                address: new Uint8Array(
                    validityProof.leaves[i].toArray('le', 32),
                ),
                root: new Uint8Array(validityProof.roots[i].toArray('le', 32)),
                rootIndex: validityProof.rootIndices[i],
                treeInfo,
            });
        }
    }

    return {
        proof: validityProof.compressedProof,
        accounts,
        addresses,
    };
}

/**
 * Packs tree infos from ValidityProofWithContext into packed format. This is a
 * TypeScript equivalent of the Rust pack_tree_infos method.
 *
 * @param validityProof - The validity proof with context (flat format)
 * @param packedAccounts - The packed accounts manager
 * @returns Packed tree infos
 */
export function packTreeInfos(
    validityProof: ValidityProofWithContext,
    packedAccounts: PackedAccounts,
): PackedTreeInfos;

/**
 * Packs tree infos from ValidityProofWithContextV2 into packed format. This is
 * a TypeScript equivalent of the Rust pack_tree_infos method.
 *
 * @param validityProof - The validity proof with context (structured format)
 * @param packedAccounts - The packed accounts manager
 * @returns Packed tree infos
 */
export function packTreeInfos(
    validityProof: ValidityProofWithContextV2,
    packedAccounts: PackedAccounts,
): PackedTreeInfos;

export function packTreeInfos(
    validityProof: ValidityProofWithContext | ValidityProofWithContextV2,
    packedAccounts: PackedAccounts,
): PackedTreeInfos {
    // Convert flat format to structured format if needed
    const structuredProof =
        'accounts' in validityProof
            ? (validityProof as ValidityProofWithContextV2)
            : convertValidityProofToV2(
                  validityProof as ValidityProofWithContext,
              );
    const packedTreeInfos: PackedStateTreeInfo[] = [];
    const addressTrees: PackedAddressTreeInfo[] = [];
    let outputTreeIndex: number | null = null;

    // Process state tree accounts
    for (const account of structuredProof.accounts) {
        // Pack TreeInfo
        const merkleTreePubkeyIndex = packedAccounts.insertOrGet(
            account.treeInfo.tree,
        );
        const queuePubkeyIndex = packedAccounts.insertOrGet(
            account.treeInfo.queue,
        );

        const treeInfoPacked: PackedStateTreeInfo = {
            rootIndex: account.rootIndex.rootIndex,
            merkleTreePubkeyIndex,
            queuePubkeyIndex,
            leafIndex: account.leafIndex,
            proveByIndex: account.rootIndex.proofByIndex,
        };
        packedTreeInfos.push(treeInfoPacked);

        // Determine output tree index
        // If a next Merkle tree exists, the Merkle tree is full -> use the next Merkle tree for new state.
        // Else use the current Merkle tree for new state.
        if (account.treeInfo.nextTreeInfo) {
            // SAFETY: account will always have a state Merkle tree context.
            // packOutputTreeIndex only throws on an invalid address Merkle tree context.
            const index = packOutputTreeIndex(
                account.treeInfo.nextTreeInfo,
                packedAccounts,
            );
            if (outputTreeIndex === null) {
                outputTreeIndex = index;
            }
        } else {
            // SAFETY: account will always have a state Merkle tree context.
            // packOutputTreeIndex only throws on an invalid address Merkle tree context.
            const index = packOutputTreeIndex(account.treeInfo, packedAccounts);
            if (outputTreeIndex === null) {
                outputTreeIndex = index;
            }
        }
    }

    // Process address tree accounts
    for (const address of structuredProof.addresses) {
        // Pack AddressTreeInfo
        const addressMerkleTreePubkeyIndex = packedAccounts.insertOrGet(
            address.treeInfo.tree,
        );
        const addressQueuePubkeyIndex = packedAccounts.insertOrGet(
            address.treeInfo.queue,
        );

        addressTrees.push({
            addressMerkleTreePubkeyIndex,
            addressQueuePubkeyIndex,
            rootIndex: address.rootIndex,
        });
    }

    // Create final packed tree infos
    const stateTrees =
        packedTreeInfos.length === 0
            ? null
            : {
                  packedTreeInfos,
                  outputTreeIndex: outputTreeIndex!,
              };

    return {
        stateTrees,
        addressTrees,
    };
}

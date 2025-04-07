import { Connection, PublicKey } from '@solana/web3.js';
import { StateTreeInfo, TreeType } from '../state/types';

/**
 * @deprecated use {@link selectStateTreeInfo} instead. Get a random tree and
 * queue from the active state tree addresses.
 *
 * Prevents write lock contention on state trees.
 *
 * @param info The active state tree addresses
 * @returns A random tree and queue
 */
export function pickRandomTreeAndQueue(info: StateTreeInfo[]): {
    tree: PublicKey;
    queue: PublicKey;
} {
    const length = info.length;
    const index = Math.floor(Math.random() * length);

    if (!info[index].queue) {
        throw new Error('Queue must not be null for state tree');
    }
    return {
        tree: info[index].tree,
        queue: info[index].queue,
    };
}

/**
 * Get a random State tree and context from the active state tree addresses.
 *
 * Prevents write lock contention on state trees.
 *
 * @param info      The active state tree addresses
 * @param treeType  The type of tree. Defaults to TreeType.StateV2
 * @returns A random tree and queue
 */
export function selectStateTreeInfo(
    info: StateTreeInfo[],
    treeType: TreeType = TreeType.StateV1,
): StateTreeInfo {
    const filteredInfo = info.filter(t => t.treeType === treeType);
    const length = filteredInfo.length;
    const index = Math.floor(Math.random() * length);

    if (!filteredInfo[index].queue) {
        throw new Error('Queue must not be null for state tree');
    }

    return filteredInfo[index];
}

/**
 * Get most recent active state tree data we store in lookup table for each
 * public state tree
 */
export async function getActiveStateTreeInfos({
    connection,
    stateTreeLookupTableAddress,
    nullifyTableAddress,
}: {
    connection: Connection;
    stateTreeLookupTableAddress: PublicKey;
    nullifyTableAddress: PublicKey;
}): Promise<StateTreeInfo[]> {
    const stateTreeLookupTable = await connection.getAddressLookupTable(
        stateTreeLookupTableAddress,
    );

    if (!stateTreeLookupTable.value) {
        throw new Error('State tree lookup table not found');
    }

    if (stateTreeLookupTable.value.state.addresses.length % 3 !== 0) {
        throw new Error(
            'State tree lookup table must have a multiple of 3 addresses',
        );
    }

    const nullifyTable =
        await connection.getAddressLookupTable(nullifyTableAddress);
    if (!nullifyTable.value) {
        throw new Error('Nullify table not found');
    }
    const stateTreePubkeys = stateTreeLookupTable.value.state.addresses;
    const nullifyTablePubkeys = nullifyTable.value.state.addresses;

    const contexts: StateTreeInfo[] = [];

    for (let i = 0; i < stateTreePubkeys.length; i += 3) {
        const tree = stateTreePubkeys[i];
        // Skip rolledover (full or almost full) Merkle trees
        if (!nullifyTablePubkeys.includes(tree)) {
            contexts.push({
                tree,
                queue: stateTreePubkeys[i + 1],
                cpiContext: stateTreePubkeys[i + 2],
                treeType: TreeType.StateV1,
            });
        }
    }

    return contexts;
}

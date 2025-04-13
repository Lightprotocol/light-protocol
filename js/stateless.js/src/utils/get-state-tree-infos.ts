import { Connection, PublicKey } from '@solana/web3.js';
import { StateTreeInfo, TreeType } from '../state/types';
import { StateTreeLUTPair } from '../constants';

/**
 * @deprecated use {@link selectStateTreeInfo} instead.
 *
 * Get a random tree and queue from a set of provided state tree infos.
 *
 * @param infos Set of state tree infos
 * @returns A random tree and queue
 */
export function pickRandomTreeAndQueue(infos: StateTreeInfo[]): {
    tree: PublicKey;
    queue: PublicKey;
} {
    const length = infos.length;
    const index = Math.floor(Math.random() * length);

    if (!infos[index].queue) {
        throw new Error('Queue must not be null for state tree');
    }
    return {
        tree: infos[index].tree,
        queue: infos[index].queue,
    };
}

/**
 * Get a pseudo-random State tree info from the set of provided state tree
 * infos.
 *
 * Using this mitigates write lock contention on state trees.
 *
 * @param info      Set of state tree infos
 * @param treeType  The type of tree. Defaults to TreeType.StateV1
 * @returns A pseudo-random tree info
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
 * Get active state tree infos from LUTs.
 *
 * @param connection            The connection to the cluster
 * @param stateTreeLUTPairs     The state tree lookup table pairs
 *
 * @returns The active state tree infos
 */
export async function getActiveStateTreeInfos({
    connection,
    stateTreeLUTPairs,
}: {
    connection: Connection;
    stateTreeLUTPairs: StateTreeLUTPair[];
}): Promise<StateTreeInfo[]> {
    const stateTreeLookupTablesAndNullifyLookupTables = await Promise.all(
        stateTreeLUTPairs.map(async lutPair => {
            return {
                stateTreeLookupTable: await connection.getAddressLookupTable(
                    lutPair.stateTreeLookupTable,
                ),
                nullifyLookupTable: await connection.getAddressLookupTable(
                    lutPair.nullifyLookupTable,
                ),
            };
        }),
    );

    const contexts: StateTreeInfo[] = [];

    for (const {
        stateTreeLookupTable,
        nullifyLookupTable,
    } of stateTreeLookupTablesAndNullifyLookupTables) {
        if (!stateTreeLookupTable.value) {
            throw new Error('State tree lookup table not found');
        }

        if (!nullifyLookupTable.value) {
            throw new Error('Nullify table not found');
        }

        const stateTreePubkeys = stateTreeLookupTable.value.state.addresses;
        const nullifyLookupTablePubkeys =
            nullifyLookupTable.value.state.addresses;

        if (stateTreePubkeys.length % 3 !== 0) {
            throw new Error(
                'State tree lookup table must have a multiple of 3 addresses',
            );
        }

        for (let i = 0; i < stateTreePubkeys.length; i += 3) {
            const tree = stateTreePubkeys[i];
            const queue = stateTreePubkeys[i + 1];
            const cpiContext = stateTreePubkeys[i + 2];

            if (!tree || !queue || !cpiContext) {
                throw new Error('Invalid state tree pubkeys structure');
            }

            // Skip rolledover (full or almost full) Merkle trees
            if (!nullifyLookupTablePubkeys.includes(tree)) {
                contexts.push({
                    tree,
                    queue,
                    cpiContext,
                    treeType: TreeType.StateV1,
                });
            }
        }
    }

    return contexts;
}

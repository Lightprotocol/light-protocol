import { Connection, PublicKey } from '@solana/web3.js';
import { TreeInfo, TreeType } from '../state/types';
import { MerkleContext } from '../state/compressed-account';
import { featureFlags, StateTreeLUTPair } from '../constants';

/**
 * Get the currently active output queue from a merkle context.
 *
 * @param merkleContext The merkle context to get the output queue from
 * @returns The output queue public key
 */
export function getOutputQueue(merkleContext: MerkleContext): PublicKey {
    return (
        merkleContext.treeInfo.nextTreeInfo?.queue ??
        merkleContext.treeInfo.queue
    );
}

/**
 * Get the currently active output tree info from a merkle context.
 *
 * @param merkleContext The merkle context to get the output tree info from
 * @returns The output tree info
 */
export function getOutputTreeInfo(merkleContext: MerkleContext): TreeInfo {
    return merkleContext.treeInfo.nextTreeInfo ?? merkleContext.treeInfo;
}

/**
 * @deprecated use {@link getTreeInfoByPubkey} instead
 */
export function getStateTreeInfoByPubkey(
    treeInfos: TreeInfo[],
    treeOrQueue: PublicKey,
): TreeInfo {
    return getTreeInfoByPubkey(treeInfos, treeOrQueue);
}

export function getTreeInfoByPubkey(
    treeInfos: TreeInfo[],
    treeOrQueue: PublicKey,
): TreeInfo {
    const treeInfo = treeInfos.find(
        info => info.tree.equals(treeOrQueue) || info.queue.equals(treeOrQueue),
    );
    if (!treeInfo) {
        throw new Error(
            `No associated TreeInfo found for tree or queue. Please set activeStateTreeInfos with latest Tree accounts. If you use custom state trees, set manually. Pubkey: ${treeOrQueue.toBase58()}`,
        );
    }
    if (!treeInfo.queue) {
        throw new Error(
            'Queue must not be null for state tree. Please set activeStateTreeInfos with latest Tree accounts. If you use custom state trees, set manually. Pubkey: ' +
                treeOrQueue.toBase58(),
        );
    }

    return treeInfo;
}

/**
 * @deprecated use {@link selectStateTreeInfo} instead.
 *
 * Get a random tree and queue from a set of provided state tree infos.
 *
 * @param infos Set of state tree infos
 * @returns A random tree and queue
 */
export function pickRandomTreeAndQueue(infos: TreeInfo[]): {
    tree: PublicKey;
    queue: PublicKey;
} {
    const length = infos.length;
    const index = Math.floor(Math.random() * length);

    let selectedIndex: number;
    if (index !== undefined) {
        if (index < 0 || index >= infos.length) {
            throw new Error(
                `Index ${index} out of bounds for infos array of length ${infos.length}`,
            );
        }
        selectedIndex = index;
    } else {
        selectedIndex = Math.floor(Math.random() * infos.length);
    }

    return infos[selectedIndex];
}

const MAX_HOTSPOTS = 5;

/**
 * Select a pseudo-random active state tree info from the set of provided state
 * tree infos.
 *
 * Using this reduces write-lock contention on state trees.
 *
 * @param infos                 Set of state tree infos
 *
 * @param treeType              Optional: Only use if you know what you are
 *                              doing. The type of tree.
 * @param useMaxConcurrency     Optional: Only use if you know what you are
 *                              doing. If true, select from all infos.
 *
 * @returns A pseudo-randomly selected tree info
 */
export function selectStateTreeInfo(
    infos: TreeInfo[],
    treeType: TreeType = featureFlags.isV2()
        ? TreeType.StateV2
        : TreeType.StateV1,
    useMaxConcurrency: boolean = false,
): TreeInfo {
    const activeInfos = infos.filter(t => !t.nextTreeInfo);
    const filteredInfos = activeInfos.filter(t => t.treeType === treeType);

    if (filteredInfos.length === 0) {
        throw new Error(
            'No active state tree infos found for the specified tree type',
        );
    }

    const length = useMaxConcurrency
        ? filteredInfos.length
        : Math.min(MAX_HOTSPOTS, filteredInfos.length);
    const index = Math.floor(Math.random() * length);

    if (!filteredInfos[index].queue) {
        throw new Error('Queue must not be null for state tree');
    }

    return filteredInfos[index];
}

/**
 * Get active state tree infos from LUTs.
 *
 * @param connection            The connection to the cluster
 * @param stateTreeLUTPairs     The state tree lookup table pairs
 *
 * @returns The active state tree infos
 */
export async function getAllStateTreeInfos({
    connection,
    stateTreeLUTPairs,
}: {
    connection: Connection;
    stateTreeLUTPairs: StateTreeLUTPair[];
}): Promise<TreeInfo[]> {
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

    const contexts: TreeInfo[] = [];

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
            let nextTreeInfo: TreeInfo | null = null;

            if (!tree || !queue || !cpiContext) {
                throw new Error('Invalid state tree pubkeys structure');
            }
            // Detect tree type based on tree address prefix
            // - bmt = batch merkle tree (StateV2)
            // - amt = address merkle tree (AddressV2 if starts with amt2, AddressV1 if amt1)
            // - smt = sparse merkle tree (StateV1)
            const treeStr = tree.toBase58();
            let treeType: TreeType;
            if (treeStr.startsWith('bmt')) {
                treeType = TreeType.StateV2;
            } else if (treeStr.startsWith('amt2')) {
                treeType = TreeType.AddressV2;
            } else if (treeStr.startsWith('amt')) {
                treeType = TreeType.AddressV1;
            } else {
                treeType = TreeType.StateV1;
            }

            if (
                nullifyLookupTablePubkeys
                    .map(addr => addr.toBase58())
                    .includes(tree.toBase58())
            ) {
                // we assign a valid tree later
                nextTreeInfo = {
                    tree: PublicKey.default,
                    queue: PublicKey.default,
                    cpiContext: PublicKey.default,
                    treeType,
                    nextTreeInfo: null,
                };
            }
            contexts.push({
                tree,
                queue,
                cpiContext,
                treeType,
                nextTreeInfo,
            });
        }

        /// for each context, check if the tree is in the nullifyLookupTable
        for (const context of contexts) {
            if (context.nextTreeInfo?.tree.equals(PublicKey.default)) {
                const nextAvailableTreeInfo = contexts.find(
                    ctx => !ctx.nextTreeInfo,
                );
                if (!nextAvailableTreeInfo) {
                    throw new Error(
                        'No available tree info found to assign as next tree',
                    );
                }
                context.nextTreeInfo = nextAvailableTreeInfo;
            }
        }
    }

    return contexts;
}

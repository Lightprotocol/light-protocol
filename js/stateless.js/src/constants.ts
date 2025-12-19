import BN from 'bn.js';
import { Buffer } from 'buffer';
import { ConfirmOptions, PublicKey } from '@solana/web3.js';
import { TreeInfo, TreeType } from './state/types';

export enum VERSION {
    V1 = 'V1',
    V2 = 'V2',
}

/**
/**
 * @internal
 * Feature flags. Only use if you know what you are doing.
 */
export const featureFlags = {
    version: ((): VERSION => {
        // Check if we're in a build environment (replaced by rollup)
        // eslint-disable-next-line no-constant-condition
        if ('__BUILD_VERSION__' !== '__BUILD_' + 'VERSION__') {
            return '__BUILD_VERSION__' as VERSION;
        }
        // Otherwise, check runtime environment variable (for tests)
        if (
            typeof process !== 'undefined' &&
            process.env?.LIGHT_PROTOCOL_VERSION
        ) {
            return process.env.LIGHT_PROTOCOL_VERSION as VERSION;
        }
        // Default to V1
        return VERSION.V1;
    })(),
    isV2: () =>
        featureFlags.version.replace(/['"]/g, '').toUpperCase() === 'V2',
};

/**
 * Returns the correct endpoint name for the current API version. E.g.
 * versionedEndpoint('getCompressedAccount') -> 'getCompressedAccount' (V1)
 * or 'getCompressedAccountV2' (V2)
 */
export const versionedEndpoint = (base: string) =>
    featureFlags.isV2() ? `${base}V2` : base;

export const FIELD_SIZE = new BN(
    '21888242871839275222246405745257275088548364400416034343698204186575808495617',
);
export const HIGHEST_ADDRESS_PLUS_ONE = new BN(
    '452312848583266388373324160190187140051835877600158453279131187530910662655',
);

export const COMPUTE_BUDGET_PATTERN = [2, 64, 66, 15, 0];

export const INVOKE_DISCRIMINATOR = Buffer.from([
    26, 16, 169, 7, 21, 202, 242, 25,
]);

export const INVOKE_CPI_DISCRIMINATOR = Buffer.from([
    49, 212, 191, 129, 39, 194, 43, 196,
]);

export const INVOKE_CPI_WITH_READ_ONLY_DISCRIMINATOR = Buffer.from([
    86, 47, 163, 166, 21, 223, 92, 8,
]);

export const INVOKE_CPI_WITH_ACCOUNT_INFO_DISCRIMINATOR = Buffer.from([
    228, 34, 128, 84, 47, 139, 86, 240,
]);

export const INSERT_INTO_QUEUES_DISCRIMINATOR = Buffer.from([
    180, 143, 159, 153, 35, 46, 248, 163,
]);

export const noopProgram = 'noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV';
export const lightSystemProgram = 'SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7';
export const accountCompressionProgram =
    'compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq';

export const getRegisteredProgramPda = () =>
    new PublicKey('35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh');

export const getAccountCompressionAuthority = () =>
    PublicKey.findProgramAddressSync(
        [Buffer.from('cpi_authority')],
        new PublicKey(lightSystemProgram),
    )[0];

export const defaultStaticAccounts = () => [
    new PublicKey(getRegisteredProgramPda()),
    new PublicKey(noopProgram),
    new PublicKey(accountCompressionProgram),
    new PublicKey(getAccountCompressionAuthority()),
];

export const defaultStaticAccountsStruct = () => {
    return {
        registeredProgramPda: new PublicKey(getRegisteredProgramPda()),
        noopProgram: new PublicKey(noopProgram),
        accountCompressionProgram: new PublicKey(accountCompressionProgram),
        accountCompressionAuthority: new PublicKey(
            getAccountCompressionAuthority(),
        ),
        cpiSignatureAccount: null,
    };
};

export type StateTreeLUTPair = {
    stateTreeLookupTable: PublicKey;
    nullifyLookupTable: PublicKey;
};

/**
 * Returns the Default Public State Tree LUTs for Devnet and Mainnet-Beta.
 */
export const defaultStateTreeLookupTables = (): {
    mainnet: StateTreeLUTPair[];
    devnet: StateTreeLUTPair[];
} => {
    return {
        mainnet: [
            {
                stateTreeLookupTable: new PublicKey(
                    stateTreeLookupTableMainnet,
                ),
                nullifyLookupTable: new PublicKey(
                    nullifiedStateTreeLookupTableMainnet,
                ),
            },
        ],
        devnet: [
            {
                stateTreeLookupTable: new PublicKey(stateTreeLookupTableDevnet),
                nullifyLookupTable: new PublicKey(
                    nullifiedStateTreeLookupTableDevnet,
                ),
            },
        ],
    };
};

/**
 * @internal
 */
export const isLocalTest = (url: string) => {
    return url.includes('localhost') || url.includes('127.0.0.1');
};

export const getDefaultAddressSpace = () => {
    return getBatchAddressTreeInfo();
};

export const getDefaultAddressTreeInfo = () => {
    if (featureFlags.isV2()) {
        return getBatchAddressTreeInfo();
    } else {
        return {
            tree: new PublicKey(addressTree),
            queue: new PublicKey(addressQueue),
            cpiContext: undefined,
            treeType: TreeType.AddressV1,
            nextTreeInfo: null,
        };
    }
};

export const getBatchAddressTreeInfo = () => {
    return {
        tree: new PublicKey(batchAddressTree),
        queue: new PublicKey(batchAddressTree),
        cpiContext: undefined,
        treeType: TreeType.AddressV2,
        nextTreeInfo: null,
    };
};
/**
 * @deprecated use {@link rpc.getStateTreeInfos} and {@link selectStateTreeInfo} instead.
 * for address trees, use {@link getDefaultAddressTreeInfo} instead.
 * Use only with Localnet testing.
 * For public networks, fetch via {@link defaultStateTreeLookupTables} and {@link getAllStateTreeInfos}.
 */
export const defaultTestStateTreeAccounts = () => {
    return {
        nullifierQueue: new PublicKey(nullifierQueuePubkey),
        merkleTree: new PublicKey(merkletreePubkey),
        merkleTreeHeight: DEFAULT_MERKLE_TREE_HEIGHT,
        addressTree: new PublicKey(addressTree),
        addressQueue: new PublicKey(addressQueue),
    };
};

/**
 * @internal testing only
 */
export const defaultTestStateTreeAccounts2 = () => {
    return {
        nullifierQueue2: new PublicKey(nullifierQueue2Pubkey),
        merkleTree2: new PublicKey(merkleTree2Pubkey),
    };
};

export const COMPRESSED_TOKEN_PROGRAM_ID = new PublicKey(
    'cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m',
);

export const CTOKEN_PROGRAM_ID = COMPRESSED_TOKEN_PROGRAM_ID;
export const stateTreeLookupTableMainnet =
    '7i86eQs3GSqHjN47WdWLTCGMW6gde1q96G2EVnUyK2st';
export const nullifiedStateTreeLookupTableMainnet =
    'H9QD4u1fG7KmkAzn2tDXhheushxFe1EcrjGGyEFXeMqT';

export const stateTreeLookupTableDevnet =
    'Dk9mNkbiZXJZ4By8DfSP6HEE4ojZzRvucwpawLeuwq8q';
export const nullifiedStateTreeLookupTableDevnet =
    'AXbHzp1NgjLvpfnD6JRTTovXZ7APUCdtWZFCRr5tCxse';

export const nullifierQueuePubkey =
    'nfq1NvQDJ2GEgnS8zt9prAe8rjjpAW1zFkrvZoBR148';
export const cpiContextPubkey = 'cpi1uHzrEhBG733DoEJNgHCyRS3XmmyVNZx5fonubE4';

export const merkletreePubkey = 'smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT';
export const addressTree = 'amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2';
export const addressQueue = 'aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F';

export const merkleTree2Pubkey = 'smt2rJAFdyJJupwMKAqTNAJwvjhmiZ4JYGZmbVRw1Ho';
export const nullifierQueue2Pubkey =
    'nfq2hgS7NYemXsFaFUCe3EMXSDSfnZnAe27jC6aPP1X';
export const cpiContext2Pubkey = 'cpi2cdhkH5roePvcudTgUL8ppEBfTay1desGh8G8QxK';

// V2
export const batchMerkleTree1 = 'bmt1LryLZUMmF7ZtqESaw7wifBXLfXHQYoE4GAmrahU';
export const batchQueue1 = 'oq1na8gojfdUhsfCpyjNt6h4JaDWtHf1yQj4koBWfto';
export const batchCpiContext1 = 'cpi15BoVPKgEPw5o8wc2T816GE7b378nMXnhH3Xbq4y';

export const batchMerkleTree2 = 'bmt2UxoBxB9xWev4BkLvkGdapsz6sZGkzViPNph7VFi';
export const batchQueue2 = 'oq2UkeMsJLfXt2QHzim242SUi3nvjJs8Pn7Eac9H9vg';
export const batchCpiContext2 = 'cpi2yGapXUR3As5SjnHBAVvmApNiLsbeZpF3euWnW6B';

export const batchMerkleTree3 = 'bmt3ccLd4bqSVZVeCJnH1F6C8jNygAhaDfxDwePyyGb';
export const batchQueue3 = 'oq3AxjekBWgo64gpauB6QtuZNesuv19xrhaC1ZM1THQ';
export const batchCpiContext3 = 'cpi3mbwMpSX8FAGMZVP85AwxqCaQMfEk9Em1v8QK9Rf';

export const batchMerkleTree4 = 'bmt4d3p1a4YQgk9PeZv5s4DBUmbF5NxqYpk9HGjQsd8';
export const batchQueue4 = 'oq4ypwvVGzCUMoiKKHWh4S1SgZJ9vCvKpcz6RT6A8dq';
export const batchCpiContext4 = 'cpi4yyPDc4bCgHAnsenunGA8Y77j3XEDyjgfyCKgcoc';

export const batchMerkleTree5 = 'bmt5yU97jC88YXTuSukYHa8Z5Bi2ZDUtmzfkDTA2mG2';
export const batchQueue5 = 'oq5oh5ZR3yGomuQgFduNDzjtGvVWfDRGLuDVjv9a96P';
export const batchCpiContext5 = 'cpi5ZTjdgYpZ1Xr7B1cMLLUE81oTtJbNNAyKary2nV6';

// V2 Address Trees
export const batchAddressTree = 'amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx';

// Deprecated: Use batchMerkleTree1, batchQueue1, batchCpiContext1 instead
export const batchMerkleTree = batchMerkleTree1;
export const batchQueue = batchQueue1;

/**
 * @internal
 * Returns local test tree infos.
 * V1: 2 state trees (smt/nfq/cpi pairs)
 * V2: 5 batched state trees (bmt/oq/cpi triplets) + 1 address tree (amt2)
 */
export const localTestActiveStateTreeInfos = (): TreeInfo[] => {
    // V1 State Trees: [tree, queue, cpi]
    const V1_STATE_TREES: [string, string, string][] = [
        [merkletreePubkey, nullifierQueuePubkey, cpiContextPubkey], // smt1, nfq1, cpi1
        [merkleTree2Pubkey, nullifierQueue2Pubkey, cpiContext2Pubkey], // smt2, nfq2, cpi2
    ];

    // V2 State Trees (batched): [bmt, oq, cpi] triplets
    const V2_STATE_TREES: [string, string, string][] = [
        [batchMerkleTree1, batchQueue1, batchCpiContext1], // bmt1, oq1, cpi1
        [batchMerkleTree2, batchQueue2, batchCpiContext2], // bmt2, oq2, cpi2
        [batchMerkleTree3, batchQueue3, batchCpiContext3], // bmt3, oq3, cpi3
        [batchMerkleTree4, batchQueue4, batchCpiContext4], // bmt4, oq4, cpi4
        [batchMerkleTree5, batchQueue5, batchCpiContext5], // bmt5, oq5, cpi5
    ];

    const V2_ADDRESS_TREE = batchAddressTree; // amt2

    const v1Trees: TreeInfo[] = V1_STATE_TREES.map(([tree, queue, cpi]) => ({
        tree: new PublicKey(tree),
        queue: new PublicKey(queue),
        cpiContext: new PublicKey(cpi),
        treeType: TreeType.StateV1,
        nextTreeInfo: null,
    }));

    const v2Trees: TreeInfo[] = V2_STATE_TREES.map(([tree, queue, cpi]) => ({
        tree: new PublicKey(tree),
        queue: new PublicKey(queue),
        cpiContext: new PublicKey(cpi),
        treeType: TreeType.StateV2,
        nextTreeInfo: null,
    }));

    const v2AddressTree: TreeInfo = {
        tree: new PublicKey(V2_ADDRESS_TREE),
        queue: new PublicKey(V2_ADDRESS_TREE), // queue is part of the tree account
        cpiContext: PublicKey.default,
        treeType: TreeType.AddressV2,
        nextTreeInfo: null,
    };

    if (featureFlags.isV2()) {
        return [...v1Trees, ...v2Trees, v2AddressTree];
    }
    return v1Trees;
};

export const confirmConfig: ConfirmOptions = {
    commitment: 'confirmed',
    preflightCommitment: 'confirmed',
};

export const DEFAULT_MERKLE_TREE_HEIGHT = 26;
export const DEFAULT_MERKLE_TREE_ROOTS = 2800;
/** Threshold (per asset) at which new in-UTXOs get merged, in order to reduce UTXO pool size */
export const UTXO_MERGE_THRESHOLD = 20;
export const UTXO_MERGE_MAXIMUM = 10;

/**
 * Treshold after which the currently used transaction Merkle tree is switched
 * to the next one
 */
export const TRANSACTION_MERKLE_TREE_ROLLOVER_THRESHOLD = new BN(
    Math.floor(2 ** DEFAULT_MERKLE_TREE_HEIGHT * 0.95),
);

/**
 * Fee to provide continous funding for the state Merkle tree.
 * Once the state Merkle tree is at 95% capacity the accumulated fees
 * will be used to fund the next state Merkle tree with the same parameters.
 *
 * Is charged per output compressed account.
 */
export const STATE_MERKLE_TREE_ROLLOVER_FEE = featureFlags.isV2()
    ? new BN(1)
    : new BN(300);

/**
 * Fee to provide continous funding for the address queue and address Merkle tree.
 * Once the address Merkle tree is at 95% capacity the accumulated fees
 * will be used to fund the next address queue and address tree with the same parameters.
 *
 * Is charged per newly created address.
 */
export const ADDRESS_QUEUE_ROLLOVER_FEE = featureFlags.isV2()
    ? new BN(392)
    : new BN(392);

/**
 * Is charged if the transaction nullifies at least one compressed account.
 */
export const STATE_MERKLE_TREE_NETWORK_FEE = new BN(5000);

/**
 * Is charged per address the transaction creates.
 */
export const ADDRESS_TREE_NETWORK_FEE_V1 = new BN(5000);

/**
 * Is charged per address the transaction creates.
 */
export const ADDRESS_TREE_NETWORK_FEE_V2 = new BN(10000);

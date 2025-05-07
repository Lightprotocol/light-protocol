import BN from 'bn.js';
import { Buffer } from 'buffer';
import { ConfirmOptions, PublicKey } from '@solana/web3.js';
import { StateTreeInfo, TreeType } from './state/types';

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

export const INSERT_INTO_QUEUES_DISCRIMINATOR = Buffer.from([
    180, 143, 159, 153, 35, 46, 248, 163,
]);

// TODO: implement properly
export const noopProgram = 'noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV';
export const lightProgram = 'SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7';
export const accountCompressionProgram = // also: merkletree program
    'compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq';

export const getRegisteredProgramPda = () =>
    new PublicKey('35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh'); // TODO: better labelling. gov authority pda

export const getAccountCompressionAuthority = () =>
    PublicKey.findProgramAddressSync(
        [Buffer.from('cpi_authority')],
        new PublicKey(
            // TODO: can add check to ensure its consistent with the idl
            lightProgram,
        ),
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

/**
 * @internal
 */
export const localTestActiveStateTreeInfo = (): StateTreeInfo[] => {
    return [
        {
            tree: new PublicKey(merkletreePubkey),
            queue: new PublicKey(nullifierQueuePubkey),
            cpiContext: new PublicKey(cpiContextPubkey),
            treeType: TreeType.StateV1,
            nextTreeInfo: null,
        },
        {
            tree: new PublicKey(merkleTree2Pubkey),
            queue: new PublicKey(nullifierQueue2Pubkey),
            cpiContext: new PublicKey(cpiContext2Pubkey),
            treeType: TreeType.StateV1,
            nextTreeInfo: null,
        },
    ];
};

export const getDefaultAddressTreeInfo = () => {
    return {
        tree: new PublicKey(addressTree),
        queue: new PublicKey(addressQueue),
        cpiContext: null,
        treeType: TreeType.AddressV1,
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
export const stateTreeLookupTableMainnet =
    '7i86eQs3GSqHjN47WdWLTCGMW6gde1q96G2EVnUyK2st';
export const nullifiedStateTreeLookupTableMainnet =
    'H9QD4u1fG7KmkAzn2tDXhheushxFe1EcrjGGyEFXeMqT';

export const stateTreeLookupTableDevnet =
    'Dk9mNkbiZXJZ4By8DfSP6HEE4ojZzRvucwpawLeuwq8q'; // '8n8rH2bFRVA6cSGNDpgqcKHCndbFCT1bXxAQG89ejVsh';
export const nullifiedStateTreeLookupTableDevnet =
    'AXbHzp1NgjLvpfnD6JRTTovXZ7APUCdtWZFCRr5tCxse'; // '5dhaJLBjnVBQFErr8oiCJmcVsx3Zj6xDekGB2zULPsnP';

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
export const STATE_MERKLE_TREE_ROLLOVER_FEE = new BN(300);

/**
 * Fee to provide continous funding for the address queue and address Merkle tree.
 * Once the address Merkle tree is at 95% capacity the accumulated fees
 * will be used to fund the next address queue and address tree with the same parameters.
 *
 * Is charged per newly created address.
 */
export const ADDRESS_QUEUE_ROLLOVER_FEE = new BN(392);

/**
 * Is charged if the transaction nullifies at least one compressed account.
 */
export const STATE_MERKLE_TREE_NETWORK_FEE = new BN(5000);

/**
 * Is charged if the transaction creates at least one address.
 */
export const ADDRESS_TREE_NETWORK_FEE = new BN(5000);

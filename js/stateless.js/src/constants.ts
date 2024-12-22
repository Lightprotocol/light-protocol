import BN from 'bn.js';
import { Buffer } from 'buffer';
import { ConfirmOptions, PublicKey } from '@solana/web3.js';

export const FIELD_SIZE = new BN(
    '21888242871839275222246405745257275088548364400416034343698204186575808495617',
);
export const HIGHEST_ADDRESS_PLUS_ONE = new BN(
    '452312848583266388373324160190187140051835877600158453279131187530910662655',
);

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

export const defaultTestStateTreeAccounts = () => {
    return {
        nullifierQueue: new PublicKey(nullifierQueuePubkey),
        merkleTree: new PublicKey(merkletreePubkey),
        merkleTreeHeight: DEFAULT_MERKLE_TREE_HEIGHT,
        addressTree: new PublicKey(addressTree),
        addressQueue: new PublicKey(addressQueue),
    };
};

export const nullifierQueuePubkey =
    'nfq1NvQDJ2GEgnS8zt9prAe8rjjpAW1zFkrvZoBR148';

export const merkletreePubkey = 'smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT';
export const addressTree = 'amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2';
export const addressQueue = 'aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F';

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

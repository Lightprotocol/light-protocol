import {
    AddressLookupTableProgram,
    Connection,
    Keypair,
    PublicKey,
    Signer,
} from '@solana/web3.js';
import { buildAndSignTx, sendAndConfirmTx } from './send-and-confirm';
import { dedupeSigner } from '../actions';
import { StateTreeInfo, TreeType } from '../state/types';
import { Rpc } from '../rpc';

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
 * Create two lookup tables storing all public state tree and queue addresses
 * returns lookup table addresses and txId
 *
 * @internal
 * @param connection    Connection to the Solana network
 * @param payer         Keypair of the payer
 * @param authority     Keypair of the authority
 * @param recentSlot    Slot of the recent block
 */
export async function createStateTreeLookupTable({
    connection,
    payer,
    authority,
    recentSlot,
}: {
    connection: Connection;
    payer: Keypair;
    authority: Keypair;
    recentSlot: number;
}): Promise<{ address: PublicKey; txId: string }> {
    const [createInstruction1, lookupTableAddress1] =
        AddressLookupTableProgram.createLookupTable({
            payer: payer.publicKey,
            authority: authority.publicKey,
            recentSlot,
        });

    const blockhash = await connection.getLatestBlockhash();

    const tx = buildAndSignTx(
        [createInstruction1],
        payer,
        blockhash.blockhash,
        dedupeSigner(payer as Signer, [authority]),
    );

    const txId = await sendAndConfirmTx(connection as Rpc, tx);

    return {
        address: lookupTableAddress1,
        txId,
    };
}

/**
 * Extend state tree lookup table with new state tree and queue addresses
 *
 * @internal
 * @param connection                Connection to the Solana network
 * @param tableAddress              Address of the lookup table to extend
 * @param newStateTreeAddresses     Addresses of the new state trees to add
 * @param newQueueAddresses         Addresses of the new queues to add
 * @param newCpiContextAddresses    Addresses of the new cpi contexts to add
 * @param payer                     Keypair of the payer
 * @param authority                 Keypair of the authority
 */
export async function extendStateTreeLookupTable({
    connection,
    tableAddress,
    newStateTreeAddresses,
    newQueueAddresses,
    newCpiContextAddresses,
    payer,
    authority,
}: {
    connection: Connection;
    tableAddress: PublicKey;
    newStateTreeAddresses: PublicKey[];
    newQueueAddresses: PublicKey[];
    newCpiContextAddresses: PublicKey[];
    payer: Keypair;
    authority: Keypair;
}): Promise<{ tableAddress: PublicKey; txId: string }> {
    const lutState = await connection.getAddressLookupTable(tableAddress);
    if (!lutState.value) {
        throw new Error('Lookup table not found');
    }
    if (lutState.value.state.addresses.length % 3 !== 0) {
        throw new Error('Lookup table must have a multiple of 3 addresses');
    }
    if (
        newStateTreeAddresses.length !== newQueueAddresses.length ||
        newStateTreeAddresses.length !== newCpiContextAddresses.length
    ) {
        throw new Error(
            'Same number of newStateTreeAddresses, newQueueAddresses, and newCpiContextAddresses required',
        );
    }

    const instructions = AddressLookupTableProgram.extendLookupTable({
        payer: payer.publicKey,
        authority: authority.publicKey,
        lookupTable: tableAddress,
        addresses: newStateTreeAddresses.flatMap((addr, index) => [
            addr,
            newQueueAddresses[index],
            newCpiContextAddresses[index],
        ]),
    });

    const blockhash = await connection.getLatestBlockhash();

    const tx = buildAndSignTx(
        [instructions],
        payer,
        blockhash.blockhash,
        dedupeSigner(payer as Signer, [authority]),
    );

    const txId = await sendAndConfirmTx(connection as Rpc, tx);

    return {
        tableAddress,
        txId,
    };
}

/**
 * Adds state tree address to lookup table. Acts as nullifier lookup for rolled
 * over state trees.
 *
 * @internal
 * @param connection                    Connection to the Solana network
 * @param stateTreeAddress              Address of the state tree to nullify
 * @param nullifyTableAddress           Address of the nullifier lookup table to
 *                                      store address in
 * @param stateTreeLookupTableAddress   lookup table storing all state tree
 *                                      addresses
 * @param payer                         Keypair of the payer
 * @param authority                     Keypair of the authority
 */
export async function nullifyLookupTable({
    connection,
    fullStateTreeAddress,
    nullifyTableAddress,
    stateTreeLookupTableAddress,
    payer,
    authority,
}: {
    connection: Connection;
    fullStateTreeAddress: PublicKey;
    nullifyTableAddress: PublicKey;
    stateTreeLookupTableAddress: PublicKey;
    payer: Keypair;
    authority: Keypair;
}): Promise<{ txId: string }> {
    // to be nullified address must be part of stateTreeLookupTable set
    const stateTreeLookupTable = await connection.getAddressLookupTable(
        stateTreeLookupTableAddress,
    );

    if (!stateTreeLookupTable.value) {
        throw new Error('State tree lookup table not found');
    }

    if (
        !stateTreeLookupTable.value.state.addresses.includes(
            fullStateTreeAddress,
        )
    ) {
        throw new Error(
            'State tree address not found in lookup table. Pass correct address or stateTreeLookupTable',
        );
    }

    const nullifyTable =
        await connection.getAddressLookupTable(nullifyTableAddress);

    if (!nullifyTable.value) {
        throw new Error('Nullify table not found');
    }
    if (nullifyTable.value.state.addresses.includes(fullStateTreeAddress)) {
        throw new Error('Address already exists in nullify lookup table');
    }

    const instructions = AddressLookupTableProgram.extendLookupTable({
        payer: payer.publicKey,
        authority: authority.publicKey,
        lookupTable: nullifyTableAddress,
        addresses: [fullStateTreeAddress],
    });

    const blockhash = await connection.getLatestBlockhash();

    const tx = buildAndSignTx([instructions], payer, blockhash.blockhash);
    // we pass a Connection type so we don't have to depend on the Rpc module.
    // @ts-expect-error
    const txId = await sendAndConfirmTx(connection, tx);

    return {
        txId,
    };
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

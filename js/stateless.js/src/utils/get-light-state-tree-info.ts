import {
    AddressLookupTableProgram,
    Connection,
    Keypair,
    PublicKey,
} from '@solana/web3.js';
import { buildAndSignTx, sendAndConfirmTx } from './send-and-confirm';
import { Rpc } from '../rpc';

/**
 * Create two lookup tables storing all public state tree and queue addresses
 * returns lookup table addresses and txId
 * [stateTreeLookupTable, nullifyTable]
 * @internal
 * @param connection - Connection to the Solana network
 * @param payer - Keypair of the payer
 * @param authority - Keypair of the authority
 * @param recentSlot - Slot of the recent block
 */
export async function createStateTreeLookupTables(
    connection: Rpc,
    payer: Keypair,
    authority: Keypair,
    recentSlot: number,
) {
    const [createInstruction1, lookupTableAddress1] =
        AddressLookupTableProgram.createLookupTable({
            payer: payer.publicKey,
            authority: authority.publicKey,
            recentSlot,
        });

    const [createInstruction2, lookupTableAddress2] =
        AddressLookupTableProgram.createLookupTable({
            payer: payer.publicKey,
            authority: authority.publicKey,
            recentSlot,
        });

    const blockhash = await connection.getLatestBlockhash();

    const tx = buildAndSignTx(
        [createInstruction1, createInstruction2],
        payer,
        blockhash.blockhash,
    );
    const txId = await sendAndConfirmTx(connection, tx);

    return {
        addresses: [lookupTableAddress1, lookupTableAddress2],
        txId,
    };
}

/**
 * Extend state tree lookup table with new state tree and queue addresses
 * @internal
 * @param connection - Connection to the Solana network
 * @param tableAddress - Address of the lookup table to extend
 * @param newStateTreeAddress - Address of the new state tree to add
 * @param newQueueAddress - Address of the new queue to add
 * @param payer - Keypair of the payer
 * @param authority - Keypair of the authority
 */
export async function extendStateTreeLookupTable(
    connection: Rpc,
    tableAddress: PublicKey,
    newStateTreeAddress: PublicKey,
    newQueueAddress: PublicKey,
    payer: Keypair,
    authority: Keypair,
) {
    const instructions = AddressLookupTableProgram.extendLookupTable({
        payer: payer.publicKey,
        authority: authority.publicKey,
        lookupTable: tableAddress,
        addresses: [newStateTreeAddress, newQueueAddress],
    });

    const blockhash = await connection.getLatestBlockhash();

    const tx = buildAndSignTx([instructions], payer, blockhash.blockhash);
    const txId = await sendAndConfirmTx(connection, tx);

    return {
        tableAddress,
        txId,
    };
}

/**
 * Adds full state tree address to lookup table. Acts as nullifier
 * @internal
 * @param connection - Connection to the Solana network
 * @param stateTreeAddress - Address of the state tree to nullify
 * @param nullifyTableAddress - Address nullifier lookup table to store address in
 * @param stateTreeLookupTableAddress - lookup table storing all state tree addresses
 * @param payer - Keypair of the payer
 * @param authority - Keypair of the authority
 */
export async function nullifyLookupTable({
    connection,
    fullStateTreeAddress,
    nullifyTableAddress,
    stateTreeLookupTableAddress,
    payer,
    authority,
}: {
    connection: Rpc;
    fullStateTreeAddress: PublicKey;
    nullifyTableAddress: PublicKey;
    stateTreeLookupTableAddress: PublicKey;
    payer: Keypair;
    authority: Keypair;
}) {
    // to be nullified address must be part of stateTreeLookupTable set
    const stateTreeLookupTable = await connection.getAddressLookupTable(
        stateTreeLookupTableAddress,
    );

    if (!stateTreeLookupTable) {
        throw new Error('State tree lookup table not found');
    }

    if (
        !stateTreeLookupTable.value?.state.addresses.includes(
            fullStateTreeAddress,
        )
    ) {
        throw new Error(
            'State tree address not found in lookup table. Pass correct address or stateTreeLookupTable',
        );
    }

    const nullifyTable =
        await connection.getAddressLookupTable(nullifyTableAddress);

    if (!nullifyTable) {
        throw new Error('Nullify table not found');
    }
    if (nullifyTable.value?.state.addresses.includes(fullStateTreeAddress)) {
        throw new Error('State tree address already in nullify table');
    }

    const instructions = AddressLookupTableProgram.extendLookupTable({
        payer: payer.publicKey,
        authority: authority.publicKey,
        lookupTable: nullifyTableAddress,
        addresses: [fullStateTreeAddress],
    });

    const blockhash = await connection.getLatestBlockhash();

    const tx = buildAndSignTx([instructions], payer, blockhash.blockhash);
    const txId = await sendAndConfirmTx(connection, tx);

    return {
        txId,
    };
}

/**
 *  Get most recent , active state tree data
 * we store in lookup table for each public state tree
 */
export async function getLightStateTreeInfo(
    connection: Connection,
    stateTreeLookupTableAddress: PublicKey,
    nullifyTableAddress: PublicKey,
) {
    const stateTreeLookupTable = await connection.getAddressLookupTable(
        stateTreeLookupTableAddress,
    );

    if (!stateTreeLookupTable) {
        throw new Error('State tree lookup table not found');
    }

    const stateTreePubkeys = stateTreeLookupTable.value?.state.addresses || [];

    const nullifyTable =
        await connection.getAddressLookupTable(nullifyTableAddress);

    if (!nullifyTable) {
        throw new Error('Nullify table not found');
    }

    const nullifyTablePubkeys = nullifyTable.value?.state.addresses || [];

    const activeStateTrees = stateTreePubkeys.filter(
        (_, index) =>
            index % 2 === 0 &&
            !nullifyTablePubkeys.includes(stateTreePubkeys[index]),
    );
    const activeQueues = stateTreePubkeys.filter(
        (_, index) =>
            index % 2 !== 0 &&
            !nullifyTablePubkeys.includes(stateTreePubkeys[index]),
    );

    if (activeStateTrees.length !== activeQueues.length) {
        throw new Error(
            'Must have equal number of active state trees and queues',
        );
    }

    return {
        activeStateTrees,
        activeQueues,
    };
}

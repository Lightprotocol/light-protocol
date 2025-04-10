import {
    PublicKey,
    Keypair,
    Connection,
    AddressLookupTableProgram,
    Signer,
} from '@solana/web3.js';
import { buildAndSignTx, sendAndConfirmTx } from './send-and-confirm';
import { dedupeSigner } from './dedupe-signer';
import { Rpc } from '../rpc';

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
 * @param nullifyLookupTableAddress     Address of the nullifier lookup table to
 *                                      store address in
 * @param stateTreeLookupTableAddress   lookup table storing all state tree
 *                                      addresses
 * @param payer                         Keypair of the payer
 * @param authority                     Keypair of the authority
 */
export async function nullifyLookupTable({
    connection,
    fullStateTreeAddress,
    nullifyLookupTableAddress,
    stateTreeLookupTableAddress,
    payer,
    authority,
}: {
    connection: Connection;
    fullStateTreeAddress: PublicKey;
    nullifyLookupTableAddress: PublicKey;
    stateTreeLookupTableAddress: PublicKey;
    payer: Keypair;
    authority: Keypair;
}): Promise<{ txId: string }> {
    // to be nullified, the address must be part of stateTreeLookupTable set
    const stateTreeLookupTable = await connection.getAddressLookupTable(
        stateTreeLookupTableAddress,
    );

    if (!stateTreeLookupTable.value) {
        console.log('stateTreeLookupTable', stateTreeLookupTable);
        throw new Error('State tree lookup table not found');
    }

    if (
        !stateTreeLookupTable.value.state.addresses
            .map(addr => addr.toBase58())
            .includes(fullStateTreeAddress.toBase58())
    ) {
        console.log('fullStateTreeAddress', fullStateTreeAddress);
        console.log(
            'stateTreeLookupTable.value.state.addresses',
            stateTreeLookupTable.value.state.addresses,
        );
        throw new Error(
            'State tree address not found in lookup table. Pass correct address or stateTreeLookupTable',
        );
    }

    const nullifyLookupTable = await connection.getAddressLookupTable(
        nullifyLookupTableAddress,
    );

    if (!nullifyLookupTable.value) {
        throw new Error('Nullify table not found');
    }
    if (
        nullifyLookupTable.value.state.addresses
            .map(addr => addr.toBase58())
            .includes(fullStateTreeAddress.toBase58())
    ) {
        throw new Error('Address already exists in nullify lookup table');
    }

    const instructions = AddressLookupTableProgram.extendLookupTable({
        payer: payer.publicKey,
        authority: authority.publicKey,
        lookupTable: nullifyLookupTableAddress,
        addresses: [fullStateTreeAddress],
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
        txId,
    };
}

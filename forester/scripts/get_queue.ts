import { Connection, PublicKey, Signer } from '@solana/web3.js';
import { newAccountWithLamports } from '@lightprotocol/stateless.js';
import { createRpc, Rpc } from '@lightprotocol/stateless.js';
import { bn, compress } from '@lightprotocol/stateless.js';
import { transfer } from '@lightprotocol/stateless.js';
import { serialize, deserialize } from 'borsh';
import { Buffer } from 'buffer';
import {
    Keypair,
    AccountMeta,
    LAMPORTS_PER_SOL,
    SystemProgram,
    Transaction,
    TransactionInstruction,
    sendAndConfirmTransaction,
} from '@solana/web3.js';
const fs = require('fs');
    
async function getNullifierQueue() {
    // load nullifier queue account data from file
    const data = fs.readFileSync('nullifierQueueAccount.data');
    console.log('Data size: ', data.length);
    const hashset = fromBytesCopy(data);
    console.log('HashSet', hashset);
}

async function saveQueueDataToFile() {
    let rpc = createRpc();
    const nullifierQueueAccountPubkey = new PublicKey("44J4oDXpjPAbzHCSc24q7NEiPekss4sAbLd8ka4gd9CZ");
    const nullifierQueueAccount = await rpc.getAccountInfo(nullifierQueueAccountPubkey);
    if (nullifierQueueAccount === null) {
        throw new Error('NullifierQueueAccount not found');
    }
    console.log('NullifierQueueAccount', nullifierQueueAccount);    
    const deserializedNullifierQueueAccount: NullifierQueueAccount = deserializeNullifierQueueAccount(nullifierQueueAccount.data.slice(8));
    console.log('Deserialized NullifierQueueAccount', deserializedNullifierQueueAccount);
    
    const merkleTreeAccountPubkey = new PublicKey("5bdFnXU47QjzGpzHfXnxcEi5WXyxzEAZzd1vrE39bf1W");
    if (deserializedNullifierQueueAccount.associatedMerkleTree.toBase58() !== merkleTreeAccountPubkey.toBase58()) {
        throw new Error('NullifierQueueAccount is not associated with the MerkleTreeAccount');
    }
    
    fs.writeFileSync('nullifierQueueAccount.data', nullifierQueueAccount.data);
    console.log('NullifierQueueAccount.data saved to nullifierQueueAccount.data');
}

getNullifierQueue().then(() => {
    console.log('NullifierQueue fetched successfully');
}).catch((error) => {
    console.error('An error occurred:', error);
});


interface NullifierQueueAccount {
    index: bigint;
    owner: PublicKey;
    delegate: PublicKey;
    associatedMerkleTree: PublicKey;
}
function deserializeNullifierQueueAccount(data: Buffer): NullifierQueueAccount {
    return {
        index: data.readBigInt64LE(0),
        owner: new PublicKey(data.slice(8, 40)),
        delegate: new PublicKey(data.slice(40, 72)),
        associatedMerkleTree: new PublicKey(data.slice(72, 104)),
    };
}

interface HashSet {
    capacity_indices: number;
    capacity_values: number;
    sequence_threshold: number;
    next_value_index: number;
    indices: Array<number | null>;
    values: Array<HashSetCell | null>;
}

interface HashSetCell {
    value: Array<number>;
    sequence_number: number;
}

function fromBytesCopy(bytes: Uint8Array): HashSet {
    const dv = new DataView(bytes.buffer);
    
    const capacity_indices = Number(dv.getBigUint64(0, true));
    const capacity_values = Number(dv.getBigUint64(8, true));
    const sequence_threshold = Number(dv.getBigUint64(16, true));
    const next_value_index = Number(dv.getBigUint64(24, true));

    console.log('capacity_indices', capacity_indices);
    console.log('capacity_values', capacity_values);
    console.log('sequence_threshold', sequence_threshold);
    console.log('next_value_index', next_value_index);
    
    const indices: Array<number | null> = Array<number | null>(
        capacity_indices,
    ).fill(null);
    const values: Array<HashSetCell | null> = Array<HashSetCell | null>(
        capacity_values,
    ).fill(null);
    
    const indicesOffset = 32 + 8; 
    for (let i = 0; i < capacity_indices; i++) {
        indices[i] = dv.getUint32(indicesOffset + i * 4, true);
    }

     const valuesOffset = indicesOffset + capacity_indices * 4;
     for (let i = 0; i < capacity_values; i++) {
        //  values[i] = {
        //      value: dv.getUint32(valuesOffset + i * 8, true),
        //      sequence_number: dv.getUint32(valuesOffset + i * 8 + 4, true),
        //  };
     }


    return {
        capacity_indices,
        capacity_values,
        sequence_threshold,
        next_value_index,
        indices,
        values,
    };
}
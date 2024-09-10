import { describe, it, assert, beforeAll } from 'vitest';
import { Keypair, Signer } from '@solana/web3.js';
import { newAccountWithLamports } from '../../src/utils/test-utils';
import { createRpc, Rpc } from '../../src/rpc';
import { getTestRpc } from '../../src/test-helpers/test-rpc';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { createStateTreeAndNullifierQueue } from '../../src/actions/create-state-tree';

describe('create-state-tree', () => {
    let rpc: Rpc;
    let payer: Signer;

    beforeAll(async () => {
        // const lightWasm = await WasmFactory.getInstance();
        
        const RPC_ENDPOINT="https://devnet.helius-rpc.com?api-key=fb5a2562-b5e7-42ec-82d2-ea9aa20a129f"

        // helius devnet
        rpc = createRpc(RPC_ENDPOINT, RPC_ENDPOINT);


        payer = await newAccountWithLamports(rpc, 6e9, 231);
        console.log(payer.publicKey.toBase58());
        console.log(payer.secretKey);
        
    });

    it('should create two state trees', async () => {
        const tx1 = await createStateTreeAndNullifierQueue(rpc, payer as Keypair, 0);
        // const tx2 = await createStateTreeAndNullifierQueue(rpc, payer as Keypair, 0);

        assert(tx1, 'Transaction 1 should be successful');
        // assert(tx2, 'Transaction 2 should be successful');
    });
});
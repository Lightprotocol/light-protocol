import { describe, it, assert, beforeAll } from 'vitest';
import { Keypair, Signer } from '@solana/web3.js';
import { newAccountWithLamports } from '../../src/utils/test-utils';
import { createRpc, Rpc } from '../../src/rpc';
import { getTestRpc } from '../../src/test-helpers/test-rpc';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { createStateTree, createAddressTree } from '../../src/actions';

describe('create-state-tree', () => {
    let rpc: Rpc;
    let payer: Signer;

    beforeAll(async () => {
        // const lightWasm = await WasmFactory.getInstance();
        
        const RPC_ENDPOINT="https://devnet.helius-rpc.com?api-key=fb5a2562-b5e7-42ec-82d2-ea9aa20a129f"

        // helius devnet
        rpc = createRpc(RPC_ENDPOINT, RPC_ENDPOINT);


        payer = await newAccountWithLamports(rpc, 1e9, 231);
        console.log(payer.publicKey.toBase58());
        console.log(payer.secretKey);
        
    });

    it.skip('should create a state tree', async () => {
        const tx = await createStateTree(rpc, payer as Keypair, 0);
        assert(tx, 'Transaction should be successful');
    });

    it('should create an address tree', async () => {
        const tx = await createAddressTree(rpc, payer as Keypair, 0);
        assert(tx, 'Transaction should be successful');
    });
});
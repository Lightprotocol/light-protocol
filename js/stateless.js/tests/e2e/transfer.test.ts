import { describe, it, assert, beforeAll } from 'vitest';
import { sendAndConfirmTx, buildAndSignTx } from '../../src/utils';

import { Keypair, Signer } from '@solana/web3.js';
import { defaultTestStateTreeAccounts } from '../../src/constants';
import {
    newAccountWithLamports,
    placeholderValidityProof,
} from '../../src/test-utils';
import { LightSystemProgram, Rpc, createRpc } from '../../src';

describe('transfer', () => {
    const { merkleTree } = defaultTestStateTreeAccounts();
    let rpc: Rpc;
    let payer: Signer;
    let bob: Signer;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc);
        bob = Keypair.generate();
    });
    /// TODO: add compression step into beforeAll.
    it('should send compressed lamports alice -> bob', async () => {
        const proof_mock = placeholderValidityProof();

        const ixs = await LightSystemProgram.transfer({
            payer: payer.publicKey,
            inputCompressedAccounts: [],
            toAddress: bob.publicKey,
            lamports: 0,
            recentInputStateRootIndices: [],
            recentValidityProof: proof_mock,
            outputStateTrees: [merkleTree, merkleTree],
        });

        /// Send
        const { blockhash } = await rpc.getLatestBlockhash();
        const signedTx = buildAndSignTx(ixs, payer, blockhash);
        await sendAndConfirmTx(rpc, signedTx);
        const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            bob.publicKey,
        );
        assert.equal(compressedAccounts.length, 1);
        assert.equal(Number(compressedAccounts[0].lamports), 0);
        assert.equal(compressedAccounts[0].data, null);
    });
});

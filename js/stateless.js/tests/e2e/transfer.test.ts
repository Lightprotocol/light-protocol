import { describe, it, assert, beforeAll } from 'vitest';
import {
    CompressedAccount,
    bn,
    createCompressedAccount,
} from '../../src/state';
import { sendAndConfirmTx, buildAndSignTx } from '../../src/utils';

import { Keypair, Signer } from '@solana/web3.js';
import { defaultTestStateTreeAccounts } from '../../src/constants';
import {
    getTestRpc,
    newAccountWithLamports,
    placeholderValidityProof,
} from '../../src/test-utils';
import { LightSystemProgram, Rpc } from '../../src';

describe('transfer', () => {
    const { merkleTree } = defaultTestStateTreeAccounts();
    let rpc: Rpc;
    let payer: Signer;
    let bob: Signer;

    beforeAll(async () => {
        rpc = await getTestRpc();
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

        rpc = await getTestRpc();

        // @ts-ignore
        const indexedEvents = await rpc.getParsedEvents();
        assert.equal(indexedEvents.length > 0, true);
        assert.equal(indexedEvents[0].inputCompressedAccounts.length, 0);
        assert.equal(indexedEvents[0].outputCompressedAccounts.length, 1);
        assert.equal(
            Number(indexedEvents[0].outputCompressedAccounts[0].lamports),
            0,
        );

        assert.equal(
            indexedEvents[0].outputCompressedAccounts[0].owner.toBase58(),
            bob.publicKey.toBase58(),
        );

        assert.equal(indexedEvents[0].outputCompressedAccounts[0].data, null);
    });
});

import { describe, it, assert, beforeAll } from 'vitest';
import { Signer } from '@solana/web3.js';
import { defaultTestStateTreeAccounts } from '../../src/constants';
import { getTestRpc, newAccountWithLamports } from '../../src/test-utils';
import {
    Rpc,
    compressLamports,
    decompressLamports,
    initSolOmnibusAccount,
} from '../../src';

/// TODO: add test case for payer != address
describe('compress', () => {
    const { merkleTree } = defaultTestStateTreeAccounts();
    let rpc: Rpc;
    let payer: Signer;
    let initAuthority: Signer;

    beforeAll(async () => {
        rpc = await getTestRpc();
        payer = await newAccountWithLamports(rpc, 1e9, 200);
        initAuthority = await newAccountWithLamports(rpc, 1e9);
    });

    it('should compress lamports and then decompress', async () => {
        const compressLamportsAmount = 20;
        const preCompressBalance = await rpc.getBalance(payer.publicKey);
        assert.equal(preCompressBalance, 1e9);

        /// TODO: add case for payer != initAuthority
        await initSolOmnibusAccount(rpc, initAuthority, initAuthority);

        await compressLamports(
            rpc,
            payer,
            compressLamportsAmount,
            payer.publicKey,
            merkleTree,
        );

        rpc = await getTestRpc();

        // @ts-ignore
        const indexedEvents = await rpc.getParsedEvents();
        assert.equal(indexedEvents.length, 2);
        assert.equal(indexedEvents[0].inputCompressedAccounts.length, 0);
        assert.equal(indexedEvents[0].outputCompressedAccounts.length, 1);
        assert.equal(
            Number(indexedEvents[0].outputCompressedAccounts[0].lamports),
            compressLamportsAmount,
        );
        assert.equal(
            indexedEvents[0].outputCompressedAccounts[0].owner.toBase58(),
            payer.publicKey.toBase58(),
        );
        assert.equal(indexedEvents[0].outputCompressedAccounts[0].data, null);
        const postCompressBalance = await rpc.getBalance(payer.publicKey);
        assert.equal(
            postCompressBalance,
            preCompressBalance - compressLamportsAmount - 5000,
        );

        /// Decompress
        const decompressLamportsAmount = 15;
        const decompressRecipient = payer.publicKey;

        await decompressLamports(
            rpc,
            payer,
            decompressLamportsAmount,
            decompressRecipient,
            merkleTree,
        );

        //@ts-ignore
        const indexedEvents2 = await rpc.getParsedEvents();
        assert.equal(indexedEvents2.length, 3);
        assert.equal(indexedEvents2[0].inputCompressedAccounts.length, 1);
        assert.equal(indexedEvents2[0].outputCompressedAccounts.length, 1);
        assert.equal(
            Number(indexedEvents2[0].outputCompressedAccounts[0].lamports),
            compressLamportsAmount - decompressLamportsAmount,
        );
        const postDecompressBalance = await rpc.getBalance(decompressRecipient);
        assert.equal(
            postDecompressBalance,
            postCompressBalance + decompressLamportsAmount - 5000,
        );
    });
});

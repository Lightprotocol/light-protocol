/// TODO: remove.
import { describe, it, assert, beforeAll } from 'vitest';
import { Signer } from '@solana/web3.js';
import { defaultTestStateTreeAccounts } from '../../src/constants';
import { newAccountWithLamports } from '../../src/test-utils';
import {
    Rpc,
    compressLamports,
    createRpc,
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
        rpc = createRpc();
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

        const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        assert.equal(compressedAccounts.length, 1);
        assert.equal(
            Number(compressedAccounts[0].lamports),
            compressLamportsAmount,
        );

        assert.equal(compressedAccounts[0].data, null);
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

        const compressedAccounts2 = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        assert.equal(compressedAccounts2.length, 1);
        assert.equal(
            Number(compressedAccounts2[0].lamports),
            compressLamportsAmount - decompressLamportsAmount,
        );
        const postDecompressBalance = await rpc.getBalance(decompressRecipient);
        assert.equal(
            postDecompressBalance,
            postCompressBalance + decompressLamportsAmount - 5000,
        );
    });
});

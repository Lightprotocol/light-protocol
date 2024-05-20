import { describe, it, assert, beforeAll, expect } from 'vitest';
import { Signer } from '@solana/web3.js';
import {
    STATE_MERKLE_TREE_ROLLOVER_FEE,
    STATE_MERKLE_TREE_TIP,
    STATE_NULLIFIER_QUEUE_ROLLOVER_FEE,
    defaultTestStateTreeAccounts,
} from '../../src/constants';
import { newAccountWithLamports } from '../../src/utils/test-utils';
import { Rpc } from '../../src/rpc';
import {
    LightSystemProgram,
    compress,
    createAccount,
    decompress,
} from '../../src';
import { TestRpc, getTestRpc } from '../../src/test-helpers/test-rpc';
import { WasmFactory } from '@lightprotocol/hasher.rs';

/// TODO: add test case for payer != address
describe('compress', () => {
    const { merkleTree } = defaultTestStateTreeAccounts();
    let rpc: Rpc;
    let payer: Signer;

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc, 1e9, 256);
    });

    it.skip('should create account with address', async () => {
        await createAccount(
            rpc as TestRpc,
            payer,
            new Uint8Array([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]),
            LightSystemProgram.programId,
        );

        await createAccount(
            rpc as TestRpc,
            payer,
            new Uint8Array([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 1,
            ]),
            LightSystemProgram.programId,
        );

        await createAccount(
            rpc as TestRpc,
            payer,
            new Uint8Array([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 2,
            ]),
            LightSystemProgram.programId,
        );
        await expect(
            createAccount(
                rpc as TestRpc,
                payer,
                new Uint8Array([
                    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                    18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 2,
                ]),
                LightSystemProgram.programId,
            ),
        ).rejects.toThrow();
    });

    it('should compress lamports and then decompress', async () => {
        payer = await newAccountWithLamports(rpc, 1e9, 256);

        const compressLamportsAmount = 1e7;
        const preCompressBalance = await rpc.getBalance(payer.publicKey);
        /// 3 createAccounts costing 5000+332 lamports each (treedepth 26)  - 5332 * 3
        assert.equal(preCompressBalance, 1e9);

        await compress(
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
            preCompressBalance -
                compressLamportsAmount -
                5000 -
                STATE_MERKLE_TREE_ROLLOVER_FEE.toNumber() -
                STATE_MERKLE_TREE_TIP.toNumber(),
        );

        
        /// Decompress
        const decompressLamportsAmount = 1e6;
        const decompressRecipient = payer.publicKey;

        await decompress(
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
            postCompressBalance +
                decompressLamportsAmount -
                5000 -
                STATE_MERKLE_TREE_ROLLOVER_FEE.toNumber() -
                STATE_NULLIFIER_QUEUE_ROLLOVER_FEE.toNumber() -
                STATE_MERKLE_TREE_TIP.toNumber() * 2, // Merkle tree and nullifier queue tip
        );
    });
});

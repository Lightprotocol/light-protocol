import { describe, it, assert, beforeAll, expect } from 'vitest';
import { Signer } from '@solana/web3.js';
import {
    STATE_MERKLE_TREE_NETWORK_FEE,
    STATE_MERKLE_TREE_ROLLOVER_FEE,
    defaultTestStateTreeAccounts,
    featureFlags,
} from '../../src/constants';
import { newAccountWithLamports } from '../../src/test-helpers/test-utils';
import { compress, decompress, transfer } from '../../src/actions';
import { bn, CompressedAccountWithMerkleContext } from '../../src/state';
import { getTestRpc, TestRpc } from '../../src/test-helpers/test-rpc';
import { WasmFactory } from '@lightprotocol/hasher.rs';

/// TODO: add test case for payer != address
describe('test-rpc', () => {
    const { merkleTree } = defaultTestStateTreeAccounts();
    let rpc: TestRpc;
    let payer: Signer;

    let preCompressBalance: number;
    let postCompressBalance: number;
    let compressLamportsAmount: number;
    let compressedTestAccount: CompressedAccountWithMerkleContext;
    let refPayer: Signer;
    const refCompressLamports = 1e7;

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);

        refPayer = await newAccountWithLamports(rpc, 1e9, 256);
        payer = await newAccountWithLamports(rpc, 1e9, 256);

        /// compress refPayer
        await compress(rpc, refPayer, refCompressLamports, refPayer.publicKey);

        /// compress
        compressLamportsAmount = 1e7;
        preCompressBalance = await rpc.getBalance(payer.publicKey);

        await compress(rpc, payer, compressLamportsAmount, payer.publicKey);
    });

    it('getCompressedAccountsByOwner', async () => {
        const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        compressedTestAccount = compressedAccounts.items[0];
        assert.equal(compressedAccounts.items.length, 1);
        assert.equal(
            Number(compressedTestAccount.lamports),
            compressLamportsAmount,
        );
        assert.equal(
            compressedTestAccount.owner.toBase58(),
            payer.publicKey.toBase58(),
        );
        assert.equal(compressedTestAccount.data?.data, null);

        postCompressBalance = await rpc.getBalance(payer.publicKey);
        let expectedFee = featureFlags.isV2()
            ? STATE_MERKLE_TREE_NETWORK_FEE.toNumber()
            : 0;
        assert.equal(
            postCompressBalance,
            preCompressBalance -
                compressLamportsAmount -
                5000 -
                expectedFee -
                STATE_MERKLE_TREE_ROLLOVER_FEE.toNumber(),
        );
    });

    it('getCompressedAccountProof for refPayer', async () => {
        const slot = await rpc.getSlot();
        const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        const refHash = compressedAccounts.items[0].hash;
        const compressedAccountProof = await rpc.getCompressedAccountProof(
            bn(refHash),
        );

        const proof = compressedAccountProof.merkleProof.map(x => x.toString());

        expect(proof.length).toStrictEqual(featureFlags.isV2() ? 32 : 26);
        expect(compressedAccountProof.hash).toStrictEqual(refHash);
        expect(compressedAccountProof.leafIndex).toStrictEqual(
            compressedAccounts.items[0].leafIndex,
        );

        preCompressBalance = await rpc.getBalance(payer.publicKey);

        await transfer(
            rpc,
            payer,
            compressLamportsAmount,
            payer,
            payer.publicKey,
        );
        const compressedAccounts1 = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        expect(compressedAccounts1.items.length).toStrictEqual(1);
        postCompressBalance = await rpc.getBalance(payer.publicKey);
        assert.equal(
            postCompressBalance,
            preCompressBalance -
                5000 -
                STATE_MERKLE_TREE_ROLLOVER_FEE.toNumber() -
                STATE_MERKLE_TREE_NETWORK_FEE.toNumber(),
        );

        await compress(rpc, payer, compressLamportsAmount, payer.publicKey);
        const compressedAccounts2 = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        expect(compressedAccounts2.items.length).toStrictEqual(2);
    });

    it('getCompressedAccountProof: get many valid proofs (10)', async () => {
        for (let lamports = 1; lamports <= 10; lamports++) {
            await decompress(rpc, payer, lamports, payer.publicKey);
        }
    });
    it('getIndexerHealth', async () => {
        /// getHealth
        const health = await rpc.getIndexerHealth();
        assert.strictEqual(health, 'ok');
    });

    it('getIndexerSlot / getSlot', async () => {
        const slot = await rpc.getIndexerSlot();
        const slotWeb3 = await rpc.getSlot();
        assert(slot > 0);
        assert(slotWeb3 > 0);
    });

    it('getCompressedAccount', async () => {
        const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        const refHash = compressedAccounts.items[0].hash;
        /// getCompressedAccount
        const compressedAccount = await rpc.getCompressedAccount(
            undefined,
            bn(refHash),
        );
        assert(compressedAccount !== null);
        assert.equal(
            compressedAccount.owner.toBase58(),
            payer.publicKey.toBase58(),
        );
        assert.equal(compressedAccount.data, null);
    });

    it('getCompressedBalance', async () => {
        const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            refPayer.publicKey,
        );
        const refHash = compressedAccounts.items[0].hash;
        /// getCompressedBalance
        await expect(rpc.getCompressedBalance(bn(refHash))).rejects.toThrow(
            'address is not supported in test-rpc',
        );

        const compressedBalance = await rpc.getCompressedBalance(
            undefined,
            bn(refHash),
        );

        expect(compressedBalance?.eq(bn(refCompressLamports))).toBeTruthy();
    });
});

import { describe, it, assert, beforeAll, expect } from 'vitest';
import { PublicKey, Signer } from '@solana/web3.js';
import {
    STATE_MERKLE_TREE_NETWORK_FEE,
    STATE_MERKLE_TREE_ROLLOVER_FEE,
} from '../../src/constants';
import { newAccountWithLamports } from '../../src/test-helpers/test-utils';
import { compress, decompress, transfer } from '../../src/actions';
import {
    bn,
    CompressedAccountWithMerkleContext,
    StateTreeInfo,
} from '../../src/state';
import { getTestRpc, TestRpc } from '../../src/test-helpers/test-rpc';
import { WasmFactory } from '@lightprotocol/hasher.rs';

describe('test-rpc V1', () => {
    let rpc: TestRpc;
    let payer: Signer;
    let outputStateTreeInfo: StateTreeInfo;
    let outputStateTreeInfoV2: StateTreeInfo;

    let preCompressBalance: number;
    let postCompressBalance: number;
    let compressLamportsAmount: number;
    let compressedTestAccount: CompressedAccountWithMerkleContext;
    let refPayer: Signer;
    const refCompressLamports = 1e7;

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);

        const stateTreeInfo = await rpc.getCachedActiveStateTreeInfos();
        outputStateTreeInfo = stateTreeInfo[0];
        outputStateTreeInfoV2 = stateTreeInfo[1];

        refPayer = await newAccountWithLamports(rpc, 1e9, 200);
        payer = await newAccountWithLamports(rpc, 1e9, 148);

        /// compress refPayer
        const id0 = await compress(
            rpc,
            refPayer,
            refCompressLamports,
            refPayer.publicKey,
            outputStateTreeInfo,
        );

        /// compress
        compressLamportsAmount = 1e7;
        preCompressBalance = await rpc.getBalance(payer.publicKey);

        const id1 = await compress(
            rpc,
            payer,
            compressLamportsAmount,
            payer.publicKey,
            outputStateTreeInfoV2,
        );
    });

    it('getCompressedAccountsByOwner', async () => {
        // refpayer
        const compressedAccountsRef = await rpc.getCompressedAccountsByOwner(
            refPayer.publicKey,
        );
        assert.equal(compressedAccountsRef.items.length, 1);
        assert.equal(compressedAccountsRef.items[0].leafIndex, 0);

        // payer
        const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        compressedTestAccount = compressedAccounts.items[0];

        assert.equal(compressedAccounts.items.length, 1);
        // assumes 1 acc per tree
        assert.equal(
            compressedTestAccount.queue.toBase58(),
            outputStateTreeInfoV2.queue!.toBase58(),
        );
        assert.equal(compressedTestAccount.leafIndex, 0);
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
        assert.equal(
            postCompressBalance,
            preCompressBalance -
                compressLamportsAmount -
                5000 -
                5000 -
                STATE_MERKLE_TREE_ROLLOVER_FEE.toNumber(),
        );
    });

    it('getCompressedAccountProof for payer', async () => {
        const slot = await rpc.getSlot();
        const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        const refHash = compressedAccounts.items[0].hash;
        const compressedAccountProof = await rpc.getCompressedAccountProof(
            bn(refHash),
        );
        const proof = compressedAccountProof.merkleProof.map(x => x.toString());

        expect(proof.length).toStrictEqual(26);
        expect(compressedAccountProof.hash).toStrictEqual(refHash);

        expect(compressedAccountProof.leafIndex).toStrictEqual(
            compressedAccounts.items[0].leafIndex,
        );
        expect(compressedAccountProof.rootIndex).toStrictEqual(1);
        preCompressBalance = await rpc.getBalance(payer.publicKey);

        // in: tree2 out: tree
        await transfer(
            rpc,
            payer,
            compressLamportsAmount,
            payer,
            payer.publicKey,
            outputStateTreeInfo,
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

        await compress(
            rpc,
            payer,
            compressLamportsAmount,
            payer.publicKey,
            outputStateTreeInfo,
        );
        const compressedAccounts2 = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        expect(compressedAccounts2.items.length).toStrictEqual(2);
    });

    it('getCompressedAccountProof: get many valid proofs (10)', async () => {
        for (let lamports = 1; lamports <= 10; lamports++) {
            const txId = await decompress(
                rpc,
                payer,
                lamports,
                payer.publicKey,
                outputStateTreeInfo,
            );
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

describe('test-rpc V2', () => {
    let rpc: TestRpc;
    let payer: Signer;
    let outputStateTreeInfo: StateTreeInfo;

    let preCompressBalance: number;
    let postCompressBalance: number;
    let compressLamportsAmount: number;
    let compressedTestAccount: CompressedAccountWithMerkleContext;
    let refPayer: Signer;
    const refCompressLamports = 1e7;

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);

        const stateTreeInfo = await rpc.getCachedActiveStateTreeInfos();
        outputStateTreeInfo = stateTreeInfo[2];

        refPayer = await newAccountWithLamports(rpc, 1e9, 256);
        payer = await newAccountWithLamports(rpc, 1e9, 256);

        /// compress refPayer
        const id0 = await compress(
            rpc,
            refPayer,
            refCompressLamports,
            refPayer.publicKey,
            outputStateTreeInfo,
        );

        /// compress
        compressLamportsAmount = 1e7;
        preCompressBalance = await rpc.getBalance(payer.publicKey);

        await compress(
            rpc,
            payer,
            compressLamportsAmount,
            payer.publicKey,
            outputStateTreeInfo,
        );
    });

    it('getCompressedAccountsByOwner', async () => {
        // refpayer
        const compressedAccountsRef = await rpc.getCompressedAccountsByOwner(
            refPayer.publicKey,
        );
        assert.equal(compressedAccountsRef.items.length, 1);

        assert.equal(compressedAccountsRef.items[0].leafIndex, 0);

        // payer
        const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        compressedTestAccount = compressedAccounts.items[0];

        assert.equal(compressedAccounts.items.length, 1);
        // assumes 1 acc per tree
        assert.equal(
            compressedTestAccount.queue.toBase58(),
            outputStateTreeInfo.queue!.toBase58(),
        );
        assert.equal(compressedTestAccount.leafIndex, 1);
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

        assert.equal(
            postCompressBalance,
            preCompressBalance - compressLamportsAmount - 5000 - 5000 - 1,
        );
    });

    it('getCompressedAccountProof for payer', async () => {
        const slot = await rpc.getSlot();
        const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        const refHash = compressedAccounts.items[0].hash;

        // Returns default merkleproof response.
        const proof = await rpc.getCompressedAccountProof(bn(refHash));
        expect(proof.treeType).toBe(0);
        expect(proof.queue).toBe(PublicKey.default);
        expect(proof.hash).toStrictEqual(new Array(32).fill(0));

        const compressedAccounts1 = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        expect(compressedAccounts1.items.length).toStrictEqual(1);

        await compress(
            rpc,
            payer,
            compressLamportsAmount,
            payer.publicKey,
            outputStateTreeInfo,
        );
        const compressedAccounts2 = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        expect(compressedAccounts2.items.length).toStrictEqual(2);
    });

    it('getCompressedAccountProof: get many valid proofs (10)', async () => {
        for (let lamports = 1; lamports <= 10; lamports++) {
            const txId = await decompress(
                rpc,
                payer,
                lamports,
                payer.publicKey,
                outputStateTreeInfo,
            );
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

        await transfer(
            rpc,
            payer,
            compressLamportsAmount,
            payer,
            payer.publicKey,
            outputStateTreeInfo,
        );
    });
});

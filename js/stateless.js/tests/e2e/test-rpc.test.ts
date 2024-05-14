import { describe, it, assert, beforeAll, expect } from 'vitest';
import { PublicKey, Signer } from '@solana/web3.js';
import {
    STATE_MERKLE_TREE_ROLLOVER_FEE,
    STATE_MERKLE_TREE_TIP,
    defaultTestStateTreeAccounts,
} from '../../src/constants';
import { newAccountWithLamports } from '../../src/utils/test-utils';
import { compress, decompress } from '../../src/actions';
import { bn, CompressedAccountWithMerkleContext } from '../../src/state';
import { getTestRpc, TestRpc } from '../../src/test-helpers/test-rpc';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { BN } from '@coral-xyz/anchor';

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

        refPayer = await newAccountWithLamports(rpc, 1e9, 200);
        payer = await newAccountWithLamports(rpc, 1e9, 148);

        /// compress refPayer
        await compress(
            rpc,
            refPayer,
            refCompressLamports,
            refPayer.publicKey,
            merkleTree,
        );

        /// compress
        compressLamportsAmount = 1e7;
        preCompressBalance = await rpc.getBalance(payer.publicKey);

        await compress(
            rpc,
            payer,
            compressLamportsAmount,
            payer.publicKey,
            merkleTree,
        );
    });

    it('getCompressedAccountsByOwner', async () => {
        const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        compressedTestAccount = compressedAccounts[0];
        assert.equal(compressedAccounts.length, 1);
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
                STATE_MERKLE_TREE_ROLLOVER_FEE.toNumber() -
                STATE_MERKLE_TREE_TIP.toNumber(),
        );
    });

    it('getCompressedAccountProof for refPayer', async () => {
        const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        const refHash = compressedAccounts[0].hash;
        const compressedAccountProof = await rpc.getCompressedAccountProof(
            bn(refHash),
        );
        const proof = compressedAccountProof.merkleProof.map(x => x.toString());

        expect(proof.length).toStrictEqual(26);
        expect(compressedAccountProof.hash).toStrictEqual(refHash);
        expect(compressedAccountProof.leafIndex).toStrictEqual(
            compressedAccounts[0].leafIndex,
        );
        expect(compressedAccountProof.rootIndex).toStrictEqual(2);

        await compress(
            rpc,
            payer,
            compressLamportsAmount,
            payer.publicKey,
            merkleTree,
        );
        const compressedAccounts1 = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        expect(compressedAccounts1?.length).toStrictEqual(2);
    });

    it('getCompressedAccountProof: get many valid proofs (10)', async () => {
        for (let lamports = 1; lamports <= 10; lamports++) {
            await decompress(rpc, payer, lamports, payer.publicKey, merkleTree);
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
        const refHash = compressedAccounts[0].hash;
        /// getCompressedAccount
        const compressedAccount = await rpc.getCompressedAccount(bn(refHash));
        assert(compressedAccount !== null);
        assert.equal(
            compressedAccount.owner.toBase58(),
            payer.publicKey.toBase58(),
        );
        assert.equal(compressedAccount.data, null);
    });

    it.skip('getCompressedBalance', async () => {
        const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        const refHash = compressedAccounts[0].hash;
        /// getCompressedBalance
        const compressedBalance = await rpc.getCompressedBalance(bn(refHash));
        expect(compressedBalance?.eq(bn(refCompressLamports))).toBeTruthy();
    });

    it.only('get Address root history array:', async () => {
        const addressMerkleTreeAccountPubkey = new PublicKey(
            'C83cpRN6oaafjNgMQJvaYgAz592EP5wunKvbokeTKPLn',
        );
        // const initedRoot = [10, 113, 73, 3, 27, 17, 230, 97, 211, 162, 174, 125, 89, 187, 79, 251, 5, 31, 40, 247, 86, 112, 152, 171, 244, 186, 109, 14, 14, 163, 48, 149];
        const initedRoot = [
            14, 189, 9, 35, 134, 65, 9, 119, 107, 233, 168, 103, 222, 227, 207,
            119, 88, 137, 200, 189, 52, 117, 226, 207, 91, 63, 70, 253, 103, 91,
            73, 117,
        ];
        let data = (await rpc.getAccountInfo(addressMerkleTreeAccountPubkey))!
            .data;
        let roots = parseAddressMerkleTreeAccounRootHistory(data);
        assert.equal(roots[3].toString(), initedRoot.toString());
        const indexOfRoot = await fetchAndSearchAddressMerkleTreeRootHistoryArray(
            rpc,
            addressMerkleTreeAccountPubkey,
            initedRoot,
        );
        assert.equal(indexOfRoot, 3);
    });
});

function parseAddressMerkleTreeAccounRootHistory(data: Buffer): number[][] {
    let startOffset = 1222136;
    let endOffset = startOffset + 76800;
    let rootData = data.subarray(startOffset, endOffset);

    let rootAccount: number[][] = [];
    let chunkSize = 32;
    for (let i = 0; i < rootData.length; i += chunkSize) {
        const root = Array.from(rootData.subarray(i, i + chunkSize));
        rootAccount.push(root);
    }
    return rootAccount;
}

async function fetchAndSearchAddressMerkleTreeRootHistoryArray(
    rpc: TestRpc,
    addressMerkleTreeAccountPubkey: PublicKey,
    root: number[] | BN,
): Promise<number> {
    let accountInfo = await rpc.getAccountInfo(addressMerkleTreeAccountPubkey);
    if (!accountInfo) {
        throw new Error("Address Merkle Tree Account does not exist.");
    }
    let roots = parseAddressMerkleTreeAccounRootHistory(accountInfo.data);
    const indexOfRoot = roots.findIndex((r) => r.toString() === root.toString());
    return indexOfRoot;
}
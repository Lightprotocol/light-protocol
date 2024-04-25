import { describe, it, assert, beforeAll, beforeEach } from 'vitest';
import { Signer } from '@solana/web3.js';
import { newAccountWithLamports } from '../../src/utils/test-utils';
import { Rpc, createRpc } from '../../src/rpc';
import {
    CompressedAccountWithMerkleContext,
    TestRpc,
    bn,
    compress,
    getTestRpc,
} from '../../src';
import { transfer } from '../../src/actions/transfer';
import { randomBytes } from 'crypto';

const selectRandomHashes = (accounts: CompressedAccountWithMerkleContext[]) => {
    const count = randomBytes(1)[0] % accounts.length;
    const shuffled = accounts.map(account => ({
        hash: account.hash,
        sort: randomBytes(1)[0],
    }));
    shuffled.sort((a, b) => a.sort - b.sort);
    return shuffled.slice(0, count).map(item => item.hash);
};

describe('rpc-interop', () => {
    let payer: Signer;
    let bob: Signer;
    /// Photon instance for debugging
    let rpc: Rpc;
    /// Mock rpc (ground truth)
    let testRpc: TestRpc;

    /// These will be set by the beforeEach hook with randomly selected values
    const transferAmount = 1e4;
    let iterations: number;
    let randomHashesPayer: number[][];
    let randomHashesBob: number[][];

    beforeAll(async () => {
        rpc = createRpc();
        testRpc = await getTestRpc();

        /// These are constant test accounts in between test runs
        payer = await newAccountWithLamports(rpc, 2e9, 112);
        bob = await newAccountWithLamports(rpc, 2e9, 113);

        await compress(rpc, payer, 1e9, payer.publicKey);
    });

    /// Execute one transfer from payer to bob and compare balances and
    /// merkleproofs via 'getCompressedAccountsByOwner' and
    /// 'getMultipleCompressedAccountProofs'.
    /// Then assign new values to 'randomHashes'
    beforeEach(async () => {
        iterations++;

        const prePayerAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        const preSenderBalance = prePayerAccounts.reduce(
            (acc, account) => acc.add(account.lamports),
            bn(0),
        );

        const preReceiverAccounts = await rpc.getCompressedAccountsByOwner(
            bob.publicKey,
        );
        const preReceiverBalance = preReceiverAccounts.reduce(
            (acc, account) => acc.add(account.lamports),
            bn(0),
        );

        /// get reference proofs for sender
        const testProofs = await testRpc.getMultipleCompressedAccountProofs(
            prePayerAccounts.map(account => bn(account.hash)),
        );

        /// get photon proofs for sender
        const proofs = await rpc.getMultipleCompressedAccountProofs(
            prePayerAccounts.map(account => bn(account.hash)),
        );
        console.log('\nTransfer', iterations + 1);

        /// compare each proof by node and root
        assert.equal(testProofs.length, proofs.length);
        proofs.forEach((proof, index) => {
            proof.merkleProof.forEach((elem, elemIndex) => {
                assert.isTrue(
                    bn(elem).eq(bn(testProofs[index].merkleProof[elemIndex])),
                );
            });
        });

        console.log('PhotonProofs', JSON.stringify(proofs));
        console.log('MockProofs', JSON.stringify(testProofs));
        assert.isTrue(bn(proofs[0].root).eq(bn(testProofs[0].root)));
        /// Note: proofs.rootIndex might be divergent if either the
        /// test-validator or photon aren't caught up with the chain state
        /// or process new txs inbetween returning the merkleproof and
        /// calling getRootSeq (since we're not getting that from photon yet
        /// (v0.11.0), we're using a mockFn called getRootSeq() in the Rpc
        /// class which fetches all events anew.

        await transfer(rpc, payer, transferAmount, payer, bob.publicKey);

        const postSenderAccs = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        const postReceiverAccs = await rpc.getCompressedAccountsByOwner(
            bob.publicKey,
        );

        const postSenderBalance = postSenderAccs.reduce(
            (acc, account) => acc.add(account.lamports),
            bn(0),
        );
        const postReceiverBalance = postReceiverAccs.reduce(
            (acc, account) => acc.add(account.lamports),
            bn(0),
        );

        assert(
            postSenderBalance.sub(preSenderBalance).eq(bn(-transferAmount)),
            `Iteration ${iterations + 1}: Sender balance should decrease by ${transferAmount}`,
        );
        assert(
            postReceiverBalance.sub(preReceiverBalance).eq(bn(transferAmount)),
            `Iteration ${iterations + 1}: Receiver balance should increase by ${transferAmount}`,
        );

        randomHashesPayer = selectRandomHashes(postSenderAccs);
        randomHashesBob = selectRandomHashes(postReceiverAccs);
    });

    it('getCompressedAccountsByOwner should match', async () => {
        const senderAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        const senderAccountsTest = await testRpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        assert.equal(senderAccounts.length, senderAccountsTest.length);

        senderAccounts.forEach((account, index) => {
            assert.equal(
                account.owner.toBase58(),
                senderAccountsTest[index].owner.toBase58(),
            );
            assert.equal(account.lamports, senderAccountsTest[index].lamports);
            assert.equal(
                account.data?.data,
                senderAccountsTest[index].data?.data,
            );
        });

        const receiverAccounts = await rpc.getCompressedAccountsByOwner(
            bob.publicKey,
        );
        const receiverAccountsTest = await testRpc.getCompressedAccountsByOwner(
            bob.publicKey,
        );

        assert.equal(receiverAccounts.length, receiverAccountsTest.length);
        receiverAccounts.forEach((account, index) => {
            assert.equal(
                account.owner.toBase58(),
                receiverAccountsTest[index].owner.toBase58(),
            );
            assert.equal(
                account.lamports,
                receiverAccountsTest[index].lamports,
            );
            assert.equal(
                account.data?.data,
                receiverAccountsTest[index].data?.data,
            );
        });
    });

    it('getCompressedAccount should match ', async () => {
        const compressedAccount = await rpc.getCompressedAccount(
            bn(randomHashesPayer[0]),
        );
        const compressedAccountTest = await testRpc.getCompressedAccount(
            bn(randomHashesPayer[0]),
        );

        assert.equal(
            compressedAccount?.lamports,
            compressedAccountTest?.lamports,
        );
        assert.equal(
            compressedAccount?.owner.toBase58(),
            compressedAccountTest?.owner.toBase58(),
        );
        assert.equal(
            compressedAccount?.data?.data,
            compressedAccountTest?.data?.data,
        );
    });

    it('getMultipleCompressedAccounts should match', async () => {
        const compressedAccounts = await rpc.getMultipleCompressedAccounts(
            randomHashesPayer.map(hash => bn(hash)),
        );
        const compressedAccountsTest =
            await testRpc.getMultipleCompressedAccounts(
                randomHashesPayer.map(hash => bn(hash)),
            );

        assert.equal(compressedAccounts.length, compressedAccountsTest.length);

        compressedAccounts.forEach((account, index) => {
            assert.equal(
                account.lamports,
                compressedAccountsTest[index].lamports,
            );
            assert.equal(
                account.owner.toBase58(),
                compressedAccountsTest[index].owner.toBase58(),
            );
            assert.equal(
                account.data?.data,
                compressedAccountsTest[index].data?.data,
            );
        });
    });
});

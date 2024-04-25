import { describe, it, assert, beforeAll } from 'vitest';
import { Signer } from '@solana/web3.js';
import { newAccountWithLamports } from '../../src/utils/test-utils';
import { Rpc, createRpc } from '../../src/rpc';
import { TestRpc, bn, compress, getTestRpc } from '../../src';
import { transfer } from '../../src/actions/transfer';

describe('rpc-interop', () => {
    let payer: Signer;
    let bob: Signer;
    /// Photon instance
    let rpc: Rpc;
    /// Mock rpc
    let testRpc: TestRpc;

    beforeAll(async () => {
        rpc = createRpc();
        testRpc = await getTestRpc();

        /// These are constant test accounts in between test runs
        payer = await newAccountWithLamports(rpc, 10e9, 256);
        bob = await newAccountWithLamports(rpc, 10e9, 256);

        await compress(rpc, payer, 1e9, payer.publicKey);
    });

    const transferAmount = 1e4;
    const numberOfTransfers = 1;
    let executedTxs = 1;

    /// FIXME: Photon returns inconsistent root / rootSeq
    it('getMultipleCompressedAccountProofs in transfer loop should match', async () => {
        for (let round = 0; round < numberOfTransfers; round++) {
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
            console.log('\nTransfer', round + 1, 'of', numberOfTransfers);

            /// compare each proof by node and root
            assert.equal(testProofs.length, proofs.length);
            proofs.forEach((proof, index) => {
                proof.merkleProof.forEach((elem, elemIndex) => {
                    assert.isTrue(
                        bn(elem).eq(
                            bn(testProofs[index].merkleProof[elemIndex]),
                        ),
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
            executedTxs++;

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
                `Iteration ${round + 1}: Sender balance should decrease by ${transferAmount}`,
            );
            assert(
                postReceiverBalance
                    .sub(preReceiverBalance)
                    .eq(bn(transferAmount)),
                `Iteration ${round + 1}: Receiver balance should increase by ${transferAmount}`,
            );
        }
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
            assert.isTrue(
                account.lamports.eq(senderAccountsTest[index].lamports),
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
            assert.isTrue(
                account.lamports.eq(receiverAccountsTest[index].lamports),
            );
        });
    });

    it('getCompressedAccount should match ', async () => {
        const senderAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        const compressedAccount = await rpc.getCompressedAccount(
            bn(senderAccounts[0].hash),
        );
        const compressedAccountTest = await testRpc.getCompressedAccount(
            bn(senderAccounts[0].hash),
        );

        assert.isTrue(
            compressedAccount!.lamports.eq(compressedAccountTest!.lamports),
        );
        assert.isTrue(
            compressedAccount!.owner.equals(compressedAccountTest!.owner),
        );
        assert.isNull(compressedAccount!.data);
        assert.isNull(compressedAccountTest!.data);
    });

    it('getMultipleCompressedAccounts should match', async () => {
        /// Emit another compressed account
        await compress(rpc, payer, 1e9, payer.publicKey);

        const senderAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        const compressedAccounts = await rpc.getMultipleCompressedAccounts(
            senderAccounts.map(account => bn(account.hash)),
        );
        const compressedAccountsTest =
            await testRpc.getMultipleCompressedAccounts(
                senderAccounts.map(account => bn(account.hash)),
            );

        assert.equal(compressedAccounts.length, compressedAccountsTest.length);

        compressedAccounts.forEach((account, index) => {
            assert.isTrue(
                account.lamports.eq(compressedAccountsTest[index].lamports),
            );
            assert.equal(
                account.owner.toBase58(),
                compressedAccountsTest[index].owner.toBase58(),
            );
            assert.isNull(account.data);
            assert.isNull(compressedAccountsTest[index].data);
        });
    });

    it('[test-rpc missing] getSignaturesForCompressedAccount should match', async () => {
        const senderAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        const signatures = await rpc.getSignaturesForCompressedAccount(
            bn(senderAccounts[0].hash),
        );

        console.log('signatures', JSON.stringify(signatures));

        assert.equal(signatures.length, 2);
    });

    it('[test-rpc missing] getSignaturesForOwner should match', async () => {
        const signatures = await rpc.getSignaturesForOwner(payer.publicKey);
        console.log(
            '@getSignaturesForOwner signatures',
            JSON.stringify(signatures),
        );
        assert.equal(signatures.length, executedTxs + 1);
    });

    /// TODO: add getCompressedTransaction, getSignaturesForAddress3
    it.skip('[test-rpc missing] getCompressedTransaction should match', async () => {
        const signatures = await rpc.getSignaturesForOwner(payer.publicKey);

        const compressedTx = await rpc.getCompressedTransaction(
            signatures[0].signature,
        );

        console.log('compressedTx', JSON.stringify(compressedTx));
        /// is compress
        assert.equal(compressedTx?.compressionInfo.closedAccounts.length, 0);
        assert.equal(compressedTx?.compressionInfo.openedAccounts.length, 1);
    });
});

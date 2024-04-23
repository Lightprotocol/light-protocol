import { describe, it, assert, beforeAll } from 'vitest';
import { Signer } from '@solana/web3.js';
import { newAccountWithLamports } from '../../src/utils/test-utils';
import { Rpc, createRpc } from '../../src/rpc';
import { bn, compress, getTestRpc } from '../../src';
import { transfer } from '../../src/actions/transfer';

describe('transfer', () => {
    let payer: Signer;
    let bob: Signer;
    /// Photon instance for debugging
    let rpc: Rpc;
    /// Mock rpc (ground truth)
    let testRpc: Rpc;

    beforeAll(async () => {
        rpc = createRpc();
        testRpc = await getTestRpc();

        /// These are constant test accounts in between test runs
        payer = await newAccountWithLamports(rpc, 2e9, 112);
        bob = await newAccountWithLamports(rpc, 2e9, 113);

        await compress(rpc, payer, 1e9, payer.publicKey);
    });

    const numberOfTransfers = 60;
    it(`should execute ${numberOfTransfers} transfers and compare merkleproofs between rpc (photon) and mockRpc`, async () => {
        const transferAmount = 1000;
        for (let i = 0; i < numberOfTransfers; i++) {
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

            /// get reference proofs
            const testProofs = await testRpc.getMultipleCompressedAccountProofs(
                preReceiverAccounts.map(account => bn(account.hash)),
            );

            /// get photon proofs
            const proofs = await rpc.getMultipleCompressedAccountProofs(
                preReceiverAccounts.map(account => bn(account.hash)),
            );

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
                `Iteration ${i + 1}: Sender balance should decrease by ${transferAmount}`,
            );
            assert(
                postReceiverBalance
                    .sub(preReceiverBalance)
                    .eq(bn(transferAmount)),
                `Iteration ${i + 1}: Receiver balance should increase by ${transferAmount}`,
            );
        }
    });
});

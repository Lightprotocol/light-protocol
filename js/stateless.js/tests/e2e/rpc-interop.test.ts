import { describe, it, assert, beforeAll } from 'vitest';
import { Signer } from '@solana/web3.js';
import { newAccountWithLamports } from '../../src/utils/test-utils';
import { Rpc, createRpc } from '../../src/rpc';
import { bn, compress } from '../../src';
import { getTestRpc, TestRpc } from '../../src/test-helpers/test-rpc';
import { transfer } from '../../src/actions/transfer';
import { WasmFactory } from '@lightprotocol/hasher.rs';

describe('rpc-interop', () => {
    let payer: Signer;
    let bob: Signer;
    let rpc: Rpc;
    let testRpc: TestRpc;
    let executedTxs = 0;
    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = createRpc();
        testRpc = await getTestRpc(lightWasm);

        /// These are constant test accounts in between test runs
        payer = await newAccountWithLamports(rpc, 10e9, 256);
        bob = await newAccountWithLamports(rpc, 10e9, 256);

        await compress(rpc, payer, 1e9, payer.publicKey);
        executedTxs++;
    });

    const transferAmount = 1e4;
    const numberOfTransfers = 15;

    it.skip('getValidityProof [noforester] (inclusion) should match', async () => {
        const senderAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        const senderAccountsTest = await testRpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        const hash = bn(senderAccounts[0].hash);
        const hashTest = bn(senderAccountsTest[0].hash);

        // accounts are the same
        assert.isTrue(hash.eq(hashTest));

        const validityProof = await rpc.getValidityProofDebug([hash]);
        const validityProofTest = await testRpc.getValidityProof([hashTest]);

        validityProof.leafIndices.forEach((leafIndex, index) => {
            assert.equal(leafIndex, validityProofTest.leafIndices[index]);
        });
        validityProof.leaves.forEach((leaf, index) => {
            assert.isTrue(leaf.eq(validityProofTest.leaves[index]));
        });
        validityProof.roots.forEach((elem, index) => {
            assert.isTrue(elem.eq(validityProofTest.roots[index]));
        });
        validityProof.rootIndices.forEach((elem, index) => {
            assert.equal(elem, validityProofTest.rootIndices[index]);
        });
        validityProof.merkleTrees.forEach((elem, index) => {
            assert.isTrue(elem.equals(validityProofTest.merkleTrees[index]));
        });
        validityProof.nullifierQueues.forEach((elem, index) => {
            assert.isTrue(
                elem.equals(validityProofTest.nullifierQueues[index]),
            );
        });

        /// FIXME: debug photon zkp
        validityProof.compressedProof.a.forEach((elem, index) => {
            const expected = validityProofTest.compressedProof.a[index];
            assert.equal(
                elem,
                expected,
                `Mismatch in compressedProof.a expected: ${validityProofTest.compressedProof.a} got: ${validityProof.compressedProof.a}`,
            );
        });

        validityProof.compressedProof.b.forEach((elem, index) => {
            const expected = validityProofTest.compressedProof.b[index];
            assert.equal(
                elem,
                expected,
                `Mismatch in compressedProof.b expected: ${validityProofTest.compressedProof.b} got: ${validityProof.compressedProof.b}`,
            );
        });

        validityProof.compressedProof.c.forEach((elem, index) => {
            const expected = validityProofTest.compressedProof.c[index];
            assert.equal(
                elem,
                expected,
                `Mismatch in compressedProof.c expected: ${validityProofTest.compressedProof.c} got: ${validityProof.compressedProof.c}`,
            );
        });
    });

    it.skip('getValidityProof [noforester] (new-addresses) should match', async () => {
        const newAddress = bn(
            new Uint8Array([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 42, 42, 14, 15, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]),
        );

        const validityProof = await rpc.getValidityProofDebug([], [newAddress]);
        const validityProofTest = await testRpc.getValidityProof(
            [],
            [newAddress],
        );

        validityProof.leafIndices.forEach((leafIndex, index) => {
            assert.equal(leafIndex, validityProofTest.leafIndices[index]);
        });
        validityProof.leaves.forEach((leaf, index) => {
            assert.isTrue(leaf.eq(validityProofTest.leaves[index]));
        });
        validityProof.roots.forEach((elem, index) => {
            assert.isTrue(elem.eq(validityProofTest.roots[index]));
        });
        validityProof.rootIndices.forEach((elem, index) => {
            assert.equal(elem, validityProofTest.rootIndices[index]);
        });
        validityProof.merkleTrees.forEach((elem, index) => {
            assert.isTrue(elem.equals(validityProofTest.merkleTrees[index]));
        });
        validityProof.nullifierQueues.forEach((elem, index) => {
            assert.isTrue(
                elem.equals(validityProofTest.nullifierQueues[index]),
            );
        });

        /// FIXME: debug photon zkp
        validityProof.compressedProof.a.forEach((elem, index) => {
            const expected = validityProofTest.compressedProof.a[index];
            assert.equal(
                elem,
                expected,
                `Mismatch in compressedProof.a expected: ${validityProofTest.compressedProof.a} got: ${validityProof.compressedProof.a}`,
            );
        });

        validityProof.compressedProof.b.forEach((elem, index) => {
            const expected = validityProofTest.compressedProof.b[index];
            assert.equal(
                elem,
                expected,
                `Mismatch in compressedProof.b expected: ${validityProofTest.compressedProof.b} got: ${validityProof.compressedProof.b}`,
            );
        });

        validityProof.compressedProof.c.forEach((elem, index) => {
            const expected = validityProofTest.compressedProof.c[index];
            assert.equal(
                elem,
                expected,
                `Mismatch in compressedProof.c expected: ${validityProofTest.compressedProof.c} got: ${validityProof.compressedProof.c}`,
            );
        });
    });

    it.skip('getValidityProof [noforester] (combined) should match', async () => {
        const senderAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        const senderAccountsTest = await testRpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        const hash = bn(senderAccounts[0].hash);
        const hashTest = bn(senderAccountsTest[0].hash);

        // accounts are the same
        assert.isTrue(hash.eq(hashTest));

        const newAddress = bn(
            new Uint8Array([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 42, 42, 14, 15, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]),
        );

        const validityProof = await rpc.getValidityProofDebug(
            [hash],
            [newAddress],
        );
        const validityProofTest = await testRpc.getValidityProof(
            [hashTest],
            [newAddress],
        );

        validityProof.leafIndices.forEach((leafIndex, index) => {
            assert.equal(leafIndex, validityProofTest.leafIndices[index]);
        });
        validityProof.leaves.forEach((leaf, index) => {
            assert.isTrue(leaf.eq(validityProofTest.leaves[index]));
        });
        validityProof.roots.forEach((elem, index) => {
            assert.isTrue(elem.eq(validityProofTest.roots[index]));
        });
        validityProof.rootIndices.forEach((elem, index) => {
            assert.equal(elem, validityProofTest.rootIndices[index]);
        });
        validityProof.merkleTrees.forEach((elem, index) => {
            assert.isTrue(elem.equals(validityProofTest.merkleTrees[index]));
        });
        validityProof.nullifierQueues.forEach((elem, index) => {
            assert.isTrue(
                elem.equals(validityProofTest.nullifierQueues[index]),
            );
        });

        /// FIXME: debug photon zkp
        validityProof.compressedProof.a.forEach((elem, index) => {
            const expected = validityProofTest.compressedProof.a[index];
            assert.equal(
                elem,
                expected,
                `Mismatch in compressedProof.a expected: ${validityProofTest.compressedProof.a} got: ${validityProof.compressedProof.a}`,
            );
        });

        validityProof.compressedProof.b.forEach((elem, index) => {
            const expected = validityProofTest.compressedProof.b[index];
            assert.equal(
                elem,
                expected,
                `Mismatch in compressedProof.b expected: ${validityProofTest.compressedProof.b} got: ${validityProof.compressedProof.b}`,
            );
        });

        validityProof.compressedProof.c.forEach((elem, index) => {
            const expected = validityProofTest.compressedProof.c[index];
            assert.equal(
                elem,
                expected,
                `Mismatch in compressedProof.c expected: ${validityProofTest.compressedProof.c} got: ${validityProof.compressedProof.c}`,
            );
        });
    });

    it.skip('getMultipleNewAddressProofs [noforester] should match', async () => {
        const newAddress = bn(
            new Uint8Array([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 42, 42, 14, 15, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]),
        );
        const newAddressProof = (
            await rpc.getMultipleNewAddressProofs([newAddress])
        )[0];
        const newAddressProofTest = (
            await testRpc.getMultipleNewAddressProofs([newAddress])
        )[0];

        assert.isTrue(
            newAddressProof.indexHashedIndexedElementLeaf.eq(
                newAddressProofTest.indexHashedIndexedElementLeaf,
            ),
        );
        assert.isTrue(
            newAddressProof.leafHigherRangeValue.eq(
                newAddressProofTest.leafHigherRangeValue,
            ),
        );
        assert.isTrue(
            newAddressProof.leafIndex.eq(newAddressProofTest.leafIndex),
        );
        assert.isTrue(
            newAddressProof.leafLowerRangeValue.eq(
                newAddressProofTest.leafLowerRangeValue,
            ),
        );

        assert.isTrue(
            newAddressProof.merkleTree.equals(newAddressProofTest.merkleTree),
        );
        assert.isTrue(
            newAddressProof.nullifierQueue.equals(
                newAddressProofTest.nullifierQueue,
            ),
        );

        assert.isTrue(newAddressProof.root.eq(newAddressProofTest.root));
        assert.isTrue(newAddressProof.value.eq(newAddressProofTest.value));

        newAddressProof.merkleProofHashedIndexedElementLeaf.forEach(
            (elem, index) => {
                const expected =
                    newAddressProofTest.merkleProofHashedIndexedElementLeaf[
                        index
                    ];
                assert.equal(
                    elem,
                    expected,
                    `Mismatch in merkleProofHashedIndexedElementLeaf expected: ${newAddressProofTest.merkleProofHashedIndexedElementLeaf} got: ${newAddressProof.merkleProofHashedIndexedElementLeaf}`,
                );
            },
        );
    });

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
        await compress(rpc, payer, 1e9, payer.publicKey);
        executedTxs++;

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
        const signaturesUnspent = await rpc.getSignaturesForCompressedAccount(
            bn(senderAccounts[0].hash),
        );

        /// most recent therefore unspent account
        assert.equal(signaturesUnspent.length, 1);

        /// Note: assumes largest-first selection mechanism
        const largestAccount = senderAccounts.reduce((acc, account) =>
            account.lamports.gt(acc.lamports) ? account : acc,
        );
        await transfer(rpc, payer, 1, payer, bob.publicKey);
        executedTxs++;

        const signaturesSpent = await rpc.getSignaturesForCompressedAccount(
            bn(largestAccount.hash),
        );

        /// 1 spent account, so always 2 signatures.
        assert.equal(signaturesSpent.length, 2);
    });

    it('[test-rpc missing] getSignaturesForOwner should match', async () => {
        const signatures = await rpc.getCompressionSignaturesForOwner(
            payer.publicKey,
        );
        assert.equal(signatures.length, executedTxs);
    });

    /// TODO: add getCompressedTransaction, getSignaturesForAddress3
    it.skip('[test-rpc missing] getCompressedTransaction should match', async () => {
        const signatures = await rpc.getCompressionSignaturesForOwner(
            payer.publicKey,
        );

        const compressedTx = await rpc.getTransactionWithCompressionInfo(
            signatures[0].signature,
        );

        /// is compress
        assert.equal(compressedTx?.compressionInfo.closedAccounts.length, 0);
        assert.equal(compressedTx?.compressionInfo.openedAccounts.length, 1);
    });
});

import { describe, it, assert, beforeAll, expect } from 'vitest';
import { PublicKey, Signer } from '@solana/web3.js';
import { newAccountWithLamports } from '../../src/test-helpers/test-utils';
import { Rpc, createRpc } from '../../src/rpc';
import {
    LightSystemProgram,
    TreeInfo,
    bn,
    compress,
    createAccount,
    createAccountWithLamports,
    deriveAddress,
    deriveAddressSeed,
    featureFlags,
    getDefaultAddressTreeInfo,
    selectStateTreeInfo,
    sleep,
} from '../../src';
import { getTestRpc, TestRpc } from '../../src/test-helpers/test-rpc';
import { transfer } from '../../src/actions/transfer';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { randomBytes } from 'tweetnacl';

const log = async (
    rpc: Rpc | TestRpc,
    payer: Signer,
    prefix: string = 'rpc',
) => {
    const accounts = await rpc.getCompressedAccountsByOwner(payer.publicKey);
    console.log(`${prefix} - indexed: `, accounts.items.length);
};

// debug helper.
const logIndexed = async (
    rpc: Rpc,
    testRpc: TestRpc,
    payer: Signer,
    prefix: string = '',
) => {
    await log(testRpc, payer, `${prefix} test-rpc `);
    await log(rpc, payer, `${prefix} rpc`);
};

describe('rpc-interop', () => {
    LightSystemProgram.deriveCompressedSolPda();
    let payer: Signer;
    let bob: Signer;
    let rpc: Rpc;
    let testRpc: TestRpc;
    let executedTxs = 0;
    let stateTreeInfo: TreeInfo;
    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = createRpc();

        testRpc = await getTestRpc(lightWasm);

        /// These are constant test accounts in between test runs
        payer = await newAccountWithLamports(rpc, 10e9, 256);
        bob = await newAccountWithLamports(rpc, 10e9, 256);

        const stateTreeInfos = await rpc.getStateTreeInfos();
        stateTreeInfo = selectStateTreeInfo(stateTreeInfos);

        await compress(rpc, payer, 1e9, payer.publicKey, stateTreeInfo);

        executedTxs++;
    });

    const transferAmount = 1e4;
    const numberOfTransfers = 15;

    it('getCompressedAccountsByOwner [noforester] filter should work', async () => {
        let accs = await rpc.getCompressedAccountsByOwner(payer.publicKey, {
            filters: [
                {
                    memcmp: {
                        offset: 1,
                        bytes: '5Vf',
                    },
                },
            ],
        });
        assert.equal(accs.items.length, 0);

        accs = await rpc.getCompressedAccountsByOwner(payer.publicKey, {
            dataSlice: { offset: 1, length: 2 },
        });

        assert.equal(accs.items.length, 1);
    });

    it('getValidityProof [noforester] (inclusion) should match', async () => {
        const senderAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        const senderAccountsTest = await testRpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        const hash = bn(senderAccounts.items[0].hash);
        const hashTest = bn(senderAccountsTest.items[0].hash);

        // accounts are the same
        assert.isTrue(hash.eq(hashTest));

        const validityProof = await rpc.getValidityProof([hash]);
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

        validityProof.treeInfos.forEach((elem, index) => {
            assert.isTrue(
                elem.tree.equals(validityProofTest.treeInfos[index].tree),
            );
        });

        validityProof.treeInfos.forEach((elem, index) => {
            assert.isTrue(
                elem.queue.equals(validityProofTest.treeInfos[index].queue),
            );
        });

        /// Executes a transfer using a 'validityProof' from Photon
        await transfer(rpc, payer, 1e5, payer, bob.publicKey);
        executedTxs++;

        /// Executes a transfer using a 'validityProof' directly from a prover.
        await transfer(testRpc, payer, 1e5, payer, bob.publicKey);
        executedTxs++;
    });

    it('getValidityProof [noforester] (new-addresses) should match', async () => {
        const newAddressSeeds = [new Uint8Array(randomBytes(32))];
        const newAddressSeed = deriveAddressSeed(
            newAddressSeeds,
            LightSystemProgram.programId,
        );

        const newAddress = bn(deriveAddress(newAddressSeed).toBuffer());

        /// consistent proof metadata for same address
        const validityProof = await rpc.getValidityProof([], [newAddress]);
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
        validityProof.treeInfos.forEach((elem, index) => {
            assert.isTrue(
                elem.tree.equals(validityProofTest.treeInfos[index].tree),
            );
        });
        validityProof.treeInfos.forEach((elem, index) => {
            assert.isTrue(
                elem.queue.equals(validityProofTest.treeInfos[index].queue),
            );
        });

        /// Need a new unique address because the previous one has been created.
        const newAddressSeedsTest = [new Uint8Array(randomBytes(32))];
        /// Creates a compressed account with address using a (non-inclusion)
        /// 'validityProof' from Photon
        await createAccount(
            rpc,
            payer,
            newAddressSeedsTest,
            LightSystemProgram.programId,
            undefined,
            stateTreeInfo,
        );
        executedTxs++;

        /// Creates a compressed account with address using a (non-inclusion)
        /// 'validityProof' directly from a prover.
        await createAccount(
            testRpc,
            payer,
            newAddressSeeds,
            LightSystemProgram.programId,
            undefined,
            stateTreeInfo,
        );
        executedTxs++;
    });

    it('getValidityProof [noforester] (combined) should match', async () => {
        const senderAccountsTest = await testRpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        // wait for photon to be in sync
        await sleep(3000);
        const senderAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        const hashTest = bn(senderAccountsTest.items[0].hash);
        const hash = bn(senderAccounts.items[0].hash);

        // accounts are the same
        assert.isTrue(hash.eq(hashTest));

        const newAddressSeeds = [new Uint8Array(randomBytes(32))];
        const newAddressSeed = deriveAddressSeed(
            newAddressSeeds,
            LightSystemProgram.programId,
        );
        const newAddress = bn(deriveAddress(newAddressSeed).toBytes());

        const validityProof = await rpc.getValidityProof([hash], [newAddress]);
        const validityProofTest = await testRpc.getValidityProof(
            [hashTest],
            [newAddress],
        );

        // compressedAccountProofs should match
        const compressedAccountProof = (
            await rpc.getMultipleCompressedAccountProofs([hash])
        )[0];
        const compressedAccountProofTest = (
            await testRpc.getMultipleCompressedAccountProofs([hashTest])
        )[0];

        compressedAccountProof.merkleProof.forEach((proof, index) => {
            assert.isTrue(
                proof.eq(compressedAccountProofTest.merkleProof[index]),
            );
        });

        // newAddressProofs should match
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
            newAddressProof.nextIndex.eq(newAddressProofTest.nextIndex),
        );
        assert.isTrue(
            newAddressProof.leafLowerRangeValue.eq(
                newAddressProofTest.leafLowerRangeValue,
            ),
        );
        assert.isTrue(
            newAddressProof.treeInfo.tree.equals(
                newAddressProofTest.treeInfo.tree,
            ),
        );
        assert.isTrue(
            newAddressProof.treeInfo.queue.equals(
                newAddressProofTest.treeInfo.queue,
            ),
        );
        assert.isTrue(newAddressProof.root.eq(newAddressProofTest.root));
        assert.isTrue(newAddressProof.value.eq(newAddressProofTest.value));

        // validity proof metadata should match
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
        validityProof.treeInfos.forEach((elem, index) => {
            assert.isTrue(
                elem.tree.equals(validityProofTest.treeInfos[index].tree),
            );
        });
        validityProof.treeInfos.forEach((elem, index) => {
            assert.isTrue(
                elem.queue.equals(validityProofTest.treeInfos[index].queue),
                'Mismatch in nullifierQueues expected: ' +
                    elem +
                    ' got: ' +
                    validityProofTest.treeInfos[index].queue,
            );
        });

        /// Creates a compressed account with address and lamports using a
        /// (combined) 'validityProof' from Photon
        await createAccountWithLamports(
            rpc,
            payer,
            [new Uint8Array(randomBytes(32))],
            0,
            LightSystemProgram.programId,
            undefined,
            stateTreeInfo,
        );
        executedTxs++;
    });

    /// This assumes support for getMultipleNewAddressProofs in Photon.
    it('getMultipleNewAddressProofs [noforester] should match', async () => {
        const newAddress = bn(
            deriveAddress(
                deriveAddressSeed(
                    [new Uint8Array(randomBytes(32))],
                    LightSystemProgram.programId,
                ),
            ).toBytes(),
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
            `Mismatch in leafHigherRangeValue expected: ${newAddressProofTest.leafHigherRangeValue} got: ${newAddressProof.leafHigherRangeValue}`,
        );
        assert.isTrue(
            newAddressProof.nextIndex.eq(newAddressProofTest.nextIndex),
            `Mismatch in leafHigherRangeValue expected: ${newAddressProofTest.nextIndex} got: ${newAddressProof.nextIndex}`,
        );
        assert.isTrue(
            newAddressProof.leafLowerRangeValue.eq(
                newAddressProofTest.leafLowerRangeValue,
            ),
        );

        assert.isTrue(
            newAddressProof.treeInfo.tree.equals(
                newAddressProofTest.treeInfo.tree,
            ),
        );
        assert.isTrue(
            newAddressProof.treeInfo.queue.equals(
                newAddressProofTest.treeInfo.queue,
            ),
            `Mismatch in nullifierQueue expected: ${newAddressProofTest.treeInfo.queue} got: ${newAddressProof.treeInfo.queue}`,
        );

        assert.isTrue(newAddressProof.root.eq(newAddressProofTest.root));
        assert.isTrue(newAddressProof.value.eq(newAddressProofTest.value));

        newAddressProof.merkleProofHashedIndexedElementLeaf.forEach(
            (elem, index) => {
                const expected =
                    newAddressProofTest.merkleProofHashedIndexedElementLeaf[
                        index
                    ];
                assert.isTrue(
                    elem.eq(expected),
                    `Mismatch in merkleProofHashedIndexedElementLeaf expected: ${expected.toString()} got: ${elem.toString()}`,
                );
            },
        );
    });

    // The test is skipped for V2 because V2 proofs return 0
    // as root for elements which are not in the tree yet.
    it.skipIf(featureFlags.isV2())(
        'getMultipleCompressedAccountProofs in transfer loop should match',
        async () => {
            for (let round = 0; round < numberOfTransfers; round++) {
                const prePayerAccounts = await rpc.getCompressedAccountsByOwner(
                    payer.publicKey,
                );
                const preSenderBalance = prePayerAccounts.items.reduce(
                    (acc, account) => acc.add(account.lamports),
                    bn(0),
                );

                const preReceiverAccounts =
                    await rpc.getCompressedAccountsByOwner(bob.publicKey);
                const preReceiverBalance = preReceiverAccounts.items.reduce(
                    (acc, account) => acc.add(account.lamports),
                    bn(0),
                );

                /// get reference proofs for sender
                const testProofs =
                    await testRpc.getMultipleCompressedAccountProofs(
                        prePayerAccounts.items.map(account => bn(account.hash)),
                    );

                /// get photon proofs for sender
                const proofs = await rpc.getMultipleCompressedAccountProofs(
                    prePayerAccounts.items.map(account => bn(account.hash)),
                );

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

                await transfer(
                    rpc,
                    payer,
                    transferAmount,
                    payer,
                    bob.publicKey,
                );
                executedTxs++;
                const postSenderAccs = await rpc.getCompressedAccountsByOwner(
                    payer.publicKey,
                );
                const postReceiverAccs = await rpc.getCompressedAccountsByOwner(
                    bob.publicKey,
                );

                const postSenderBalance = postSenderAccs.items.reduce(
                    (acc, account) => acc.add(account.lamports),
                    bn(0),
                );
                const postReceiverBalance = postReceiverAccs.items.reduce(
                    (acc, account) => acc.add(account.lamports),
                    bn(0),
                );

                assert(
                    postSenderBalance
                        .sub(preSenderBalance)
                        .eq(bn(-transferAmount)),
                    `Iteration ${round + 1}: Sender balance should decrease by ${transferAmount}`,
                );
                assert(
                    postReceiverBalance
                        .sub(preReceiverBalance)
                        .eq(bn(transferAmount)),
                    `Iteration ${round + 1}: Receiver balance should increase by ${transferAmount}`,
                );
            }
        },
    );

    it('getCompressedAccountsByOwner should match', async () => {
        const senderAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        const senderAccountsTest = await testRpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        assert.equal(
            senderAccounts.items.length,
            senderAccountsTest.items.length,
        );

        senderAccounts.items.forEach((account, index) => {
            assert.equal(
                account.owner.toBase58(),
                senderAccountsTest.items[index].owner.toBase58(),
            );
            assert.isTrue(
                account.lamports.eq(senderAccountsTest.items[index].lamports),
            );
        });

        const receiverAccounts = await rpc.getCompressedAccountsByOwner(
            bob.publicKey,
        );
        const receiverAccountsTest = await testRpc.getCompressedAccountsByOwner(
            bob.publicKey,
        );

        assert.equal(
            receiverAccounts.items.length,
            receiverAccountsTest.items.length,
        );

        receiverAccounts.items.sort((a, b) =>
            a.lamports.sub(b.lamports).toNumber(),
        );
        receiverAccountsTest.items.sort((a, b) =>
            a.lamports.sub(b.lamports).toNumber(),
        );

        receiverAccounts.items.forEach((account, index) => {
            assert.equal(
                account.owner.toBase58(),
                receiverAccountsTest.items[index].owner.toBase58(),
            );
            assert.isTrue(
                account.lamports.eq(receiverAccountsTest.items[index].lamports),
            );
        });
    });

    it('getCompressedAccount should match ', async () => {
        const senderAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        const compressedAccount = await rpc.getCompressedAccount(
            undefined,
            bn(senderAccounts.items[0].hash),
        );
        const compressedAccountTest = await testRpc.getCompressedAccount(
            undefined,
            bn(senderAccounts.items[0].hash),
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
        await compress(rpc, payer, 1e9, payer.publicKey, stateTreeInfo);
        executedTxs++;

        const senderAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        const compressedAccounts = await rpc.getMultipleCompressedAccounts(
            senderAccounts.items.map(account => bn(account.hash)),
        );
        const compressedAccountsTest =
            await testRpc.getMultipleCompressedAccounts(
                senderAccounts.items.map(account => bn(account.hash)),
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

    it('[test-rpc missing] getCompressionSignaturesForAccount should match', async () => {
        const senderAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        const signaturesUnspent = await rpc.getCompressionSignaturesForAccount(
            bn(senderAccounts.items[0].hash),
        );

        /// most recent therefore unspent account
        assert.equal(signaturesUnspent.length, 1);

        /// Note: assumes largest-first selection mechanism
        const largestAccount = senderAccounts.items.reduce((acc, account) =>
            account.lamports.gt(acc.lamports) ? account : acc,
        );

        await transfer(rpc, payer, 1, payer, bob.publicKey);
        executedTxs++;

        const signaturesSpent = await rpc.getCompressionSignaturesForAccount(
            bn(largestAccount.hash),
        );

        /// 1 spent account, so always 2 signatures.
        assert.equal(signaturesSpent.length, 2);
    });

    it('[test-rpc missing] getSignaturesForOwner should match', async () => {
        const signatures = await rpc.getCompressionSignaturesForOwner(
            payer.publicKey,
        );
        assert.equal(signatures.items.length, executedTxs);
    });

    it('[test-rpc missing] getLatestNonVotingSignatures should match', async () => {
        const testEnvSetupTxs = 2;

        let signatures = (await rpc.getLatestNonVotingSignatures()).value.items;
        assert.isAtLeast(signatures.length, executedTxs + testEnvSetupTxs);

        signatures = (await rpc.getLatestNonVotingSignatures(2)).value.items;
        assert.equal(signatures.length, 2);
    });

    it('[test-rpc missing] getLatestCompressionSignatures should match', async () => {
        const { items: signatures } = (
            await rpc.getLatestCompressionSignatures()
        ).value;

        assert.isAtLeast(signatures.length, executedTxs);

        /// Should return 1 using limit param
        const { items: signatures2, cursor } = (
            await rpc.getLatestCompressionSignatures(undefined, 1)
        ).value;

        assert.equal(signatures2.length, 1);

        // wait for photon to be in sync
        await sleep(3000);
        const { items: signatures3 } = (
            await rpc.getLatestCompressionSignatures(cursor!, 1)
        ).value;

        /// cursor should workv
        assert.notEqual(signatures2[0].signature, signatures3[0].signature);
    });

    it('[test-rpc missing] getCompressedTransaction should match', async () => {
        const signatures = await rpc.getCompressionSignaturesForOwner(
            payer.publicKey,
        );

        const compressedTx = await rpc.getTransactionWithCompressionInfo(
            signatures.items[0].signature,
        );

        /// is transfer
        assert.equal(compressedTx?.compressionInfo.closedAccounts.length, 1);
        assert.equal(compressedTx?.compressionInfo.openedAccounts.length, 2);
    });

    it('[test-rpc missing] getCompressionSignaturesForAddress should work', async () => {
        const seeds = [new Uint8Array(randomBytes(32))];
        const seed = deriveAddressSeed(seeds, LightSystemProgram.programId);
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const address = deriveAddress(seed, addressTreeInfo.tree);

        await createAccount(
            rpc,
            payer,
            seeds,
            LightSystemProgram.programId,
            addressTreeInfo,
            stateTreeInfo,
        );

        const accounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        const allAccountsTestRpc = await testRpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        const allAccountsRpc = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        const latestAccount = accounts.items[0];

        // assert the address was indexed
        assert.isTrue(new PublicKey(latestAccount.address!).equals(address));

        const signaturesUnspent = await rpc.getCompressionSignaturesForAddress(
            new PublicKey(latestAccount.address!),
        );

        /// most recent therefore unspent account
        assert.equal(signaturesUnspent.items.length, 1);
    });

    it('[test-rpc missing] getCompressedAccount with address param should work ', async () => {
        const seeds = [new Uint8Array(randomBytes(32))];
        const seed = deriveAddressSeed(seeds, LightSystemProgram.programId);

        const addressTreeInfo = getDefaultAddressTreeInfo();
        const address = deriveAddress(seed, addressTreeInfo.tree);

        await createAccount(
            rpc,
            payer,
            seeds,
            LightSystemProgram.programId,
            addressTreeInfo,
            stateTreeInfo,
        );

        // fetch the owners latest account
        const accounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        const latestAccount = accounts.items[0];

        assert.isTrue(new PublicKey(latestAccount.address!).equals(address));

        const compressedAccountByHash = await rpc.getCompressedAccount(
            undefined,
            bn(latestAccount.hash),
        );
        const compressedAccountByAddress = await rpc.getCompressedAccount(
            bn(latestAccount.address!),
            undefined,
        );

        await expect(
            testRpc.getCompressedAccount(bn(latestAccount.address!), undefined),
        ).rejects.toThrow();

        assert.isTrue(
            bn(compressedAccountByHash!.address!).eq(
                bn(compressedAccountByAddress!.address!),
            ),
        );
    });
});

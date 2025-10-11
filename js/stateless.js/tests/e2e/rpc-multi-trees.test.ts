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
    selectStateTreeInfo,
} from '../../src';
import { getTestRpc, TestRpc } from '../../src/test-helpers/test-rpc';
import { transfer } from '../../src/actions/transfer';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { randomBytes } from 'tweetnacl';

describe('rpc-multi-trees', () => {
    let payer: Signer;
    let bob: Signer;
    let rpc: Rpc;
    let testRpc: TestRpc;
    let executedTxs = 0;

    const randTrees: PublicKey[] = [];
    const randQueues: PublicKey[] = [];
    let stateTreeInfo2: TreeInfo;
    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = createRpc();

        testRpc = await getTestRpc(lightWasm);

        const stateTreeInfo = selectStateTreeInfo(
            await rpc.getStateTreeInfos(),
        );
        if (featureFlags.isV2()) {
            // TODO: add test specifically for multiple v2 trees.
            stateTreeInfo2 = stateTreeInfo;
        } else
            stateTreeInfo2 = selectStateTreeInfo(await rpc.getStateTreeInfos());

        /// These are constant test accounts in between test runs
        payer = await newAccountWithLamports(rpc, 10e9, 256);
        bob = await newAccountWithLamports(rpc, 10e9, 256);

        await compress(rpc, payer, 1e9, payer.publicKey, stateTreeInfo);
        randTrees.push(stateTreeInfo.tree);
        randQueues.push(stateTreeInfo.queue);
        executedTxs++;
    });

    const transferAmount = 1e4;
    const numberOfTransfers = 15;

    it('account must have merkleTree2 and nullifierQueue2', async () => {
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

        expect(accs.items[0].treeInfo.tree).toEqual(randTrees[0]);
        expect(accs.items[0].treeInfo.queue).toEqual(randQueues[0]);

        assert.equal(accs.items.length, 1);
    });

    let address: PublicKey;
    it('must create account with random output tree (selectStateTreeInfo)', async () => {
        const tree = selectStateTreeInfo(await rpc.getStateTreeInfos());

        const seed = randomBytes(32);
        const addressSeed = deriveAddressSeed(
            [seed],
            LightSystemProgram.programId,
        );
        address = deriveAddress(addressSeed);

        await createAccount(
            rpc,
            payer,
            [seed],
            LightSystemProgram.programId,
            undefined,
            tree, // output state tree
        );

        randTrees.push(tree.tree);
        randQueues.push(tree.queue);

        const acc = await rpc.getCompressedAccount(bn(address.toBuffer()));
        expect(acc!.treeInfo.tree).toEqual(tree.tree);
        expect(acc!.treeInfo.queue).toEqual(tree.queue);
    });

    it('getValidityProof [noforester] (inclusion) should return correct trees and queues', async () => {
        const acc = await rpc.getCompressedAccount(bn(address.toBuffer()));

        const hash = bn(acc!.hash);
        const pos = randTrees.length - 1;
        expect(acc?.treeInfo.tree).toEqual(randTrees[pos]);
        expect(acc?.treeInfo.queue).toEqual(randQueues[pos]);

        const validityProof = await rpc.getValidityProof([hash]);

        expect(validityProof.treeInfos[0].tree).toEqual(randTrees[pos]);
        expect(validityProof.treeInfos[0].queue).toEqual(randQueues[pos]);

        /// Executes transfers using random output trees
        const tree1 = selectStateTreeInfo(await rpc.getStateTreeInfos());
        await transfer(rpc, payer, 1e5, payer, bob.publicKey);
        executedTxs++;
        randTrees.push(tree1.tree);
        randQueues.push(tree1.queue);

        const tree2 = selectStateTreeInfo(await rpc.getStateTreeInfos());
        await transfer(rpc, payer, 1e5, payer, bob.publicKey);
        executedTxs++;
        randTrees.push(tree2.tree);
        randQueues.push(tree2.queue);

        const validityProof2 = await rpc.getValidityProof([hash]);

        expect(validityProof2.treeInfos[0].tree).toEqual(randTrees[pos]);
        expect(validityProof2.treeInfos[0].queue).toEqual(randQueues[pos]);
    });

    it('getValidityProof [noforester] (combined) should return correct trees and queues', async () => {
        const senderAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        const hash = bn(senderAccounts.items[0].hash);

        const newAddressSeeds = [new Uint8Array(randomBytes(32))];
        const newAddressSeed = deriveAddressSeed(
            newAddressSeeds,
            LightSystemProgram.programId,
        );
        const newAddress = bn(deriveAddress(newAddressSeed).toBytes());

        const validityProof = await rpc.getValidityProof([hash], [newAddress]);

        // compressedAccountProofs should be valid
        const compressedAccountProof = (
            await rpc.getMultipleCompressedAccountProofs([hash])
        )[0];

        compressedAccountProof.merkleProof.forEach((proof, index) => {
            assert.isTrue(proof.eq(compressedAccountProof.merkleProof[index]));
        });

        // newAddressProofs should be valid
        const newAddressProof = (
            await rpc.getMultipleNewAddressProofs([newAddress])
        )[0];

        // only compare state tree
        assert.isTrue(
            validityProof.treeInfos[0].tree.equals(
                senderAccounts.items[0].treeInfo.tree,
            ),
            'Mismatch in merkleTrees expected: ' +
                senderAccounts.items[0].treeInfo.tree +
                ' got: ' +
                validityProof.treeInfos[0].tree,
        );
        assert.isTrue(
            validityProof.treeInfos[0].queue.equals(
                senderAccounts.items[0].treeInfo.queue,
            ),
            `Mismatch in nullifierQueues expected: ${senderAccounts.items[0].treeInfo.queue} got: ${validityProof.treeInfos[0].queue}`,
        );

        /// Creates a compressed account with address and lamports using a
        /// (combined) 'validityProof' from Photon
        const tree = selectStateTreeInfo(await rpc.getStateTreeInfos());
        await createAccountWithLamports(
            rpc,
            payer,
            [new Uint8Array(randomBytes(32))],
            0,
            LightSystemProgram.programId,
            undefined,
            tree,
        );
        executedTxs++;
        randTrees.push(tree.tree);
        randQueues.push(tree.queue);
    });

    it('getMultipleCompressedAccountProofs in transfer loop should match', async () => {
        for (let round = 0; round < numberOfTransfers; round++) {
            const prePayerAccounts = await rpc.getCompressedAccountsByOwner(
                payer.publicKey,
            );

            const proofs = await rpc.getMultipleCompressedAccountProofs(
                prePayerAccounts.items.map(account => bn(account.hash)),
            );

            proofs.forEach((proof, index) => {
                const expectedTree =
                    prePayerAccounts.items[index].treeInfo.tree;
                const actualTree = proof.treeInfo.tree;
                const expectedQueue =
                    prePayerAccounts.items[index].treeInfo.queue;
                const actualQueue = proof.treeInfo.queue;

                console.log(`Iteration ${round + 1}, Account ${index}:`);
                console.log(
                    `  Expected tree (from getCompressedAccountsByOwner): ${expectedTree.toBase58()}`,
                );
                console.log(
                    `  Actual tree (from getMultipleCompressedAccountProofs): ${actualTree.toBase58()}`,
                );
                console.log(`  Expected queue: ${expectedQueue.toBase58()}`);
                console.log(`  Actual queue: ${actualQueue.toBase58()}`);

                assert.isTrue(
                    actualTree.equals(expectedTree),
                    `Iteration ${round + 1}: Mismatch in merkleTree for account index ${index}`,
                );
                assert.isTrue(
                    actualQueue.equals(expectedQueue),
                    `Iteration ${round + 1}: Mismatch in nullifierQueue for account index ${index}`,
                );
            });

            const tree = selectStateTreeInfo(await rpc.getStateTreeInfos());
            console.log(
                `Selected tree for transfer in round ${round + 1}: ${tree.tree.toBase58()}`,
            );
            await transfer(rpc, payer, transferAmount, payer, bob.publicKey);
            executedTxs++;
        }
    });

    it('getMultipleCompressedAccounts should match', async () => {
        await compress(rpc, payer, 1e9, payer.publicKey, stateTreeInfo2);
        executedTxs++;

        const senderAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        const compressedAccounts = await rpc.getMultipleCompressedAccounts(
            senderAccounts.items.map(account => bn(account.hash)),
        );

        compressedAccounts.forEach((account, index) => {
            assert.isTrue(
                account.treeInfo.tree.equals(
                    senderAccounts.items[index].treeInfo.tree,
                ),
                `Mismatch in merkleTree for account index ${index}`,
            );
            assert.isTrue(
                account.treeInfo.queue.equals(
                    senderAccounts.items[index].treeInfo.queue,
                ),
                `Mismatch in nullifierQueue for account index ${index}`,
            );
        });
    });
});

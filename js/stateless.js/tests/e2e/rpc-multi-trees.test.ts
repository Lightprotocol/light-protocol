import { describe, it, assert, beforeAll, expect } from 'vitest';
import { PublicKey, Signer } from '@solana/web3.js';
import { newAccountWithLamports } from '../../src/test-helpers/test-utils';
import { Rpc, createRpc, pickRandomTreeAndQueue } from '../../src/rpc';
import {
    LightSystemProgram,
    StateTreeContext,
    bn,
    compress,
    createAccount,
    createAccountWithLamports,
    defaultTestStateTreeAccounts2,
    deriveAddress,
    deriveAddressSeed,
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

    let outputStateTreeContext: StateTreeContext;
    let outputStateTreeContext2: StateTreeContext;
    beforeAll(async () => {
        // const lightWasm = await WasmFactory.getInstance();
        // rpc = await getTestRpc(lightWasm);
        rpc = createRpc();

        const stateTreeInfo = await rpc.getCachedActiveStateTreeInfo();
        outputStateTreeContext = stateTreeInfo[0];
        outputStateTreeContext2 = stateTreeInfo[1];

        /// These are constant test accounts in between test runs
        payer = await newAccountWithLamports(rpc, 10e9, 256);
        bob = await newAccountWithLamports(rpc, 10e9, 256);

        // tree 1
        await compress(
            rpc,
            payer,
            1e9,
            payer.publicKey,
            outputStateTreeContext,
        );
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

        expect(accs.items[0].merkleTree).toEqual(outputStateTreeContext.tree);
        expect(accs.items[0].queue).toEqual(outputStateTreeContext.queue!);

        assert.equal(accs.items.length, 1);
    });

    let address: PublicKey;
    it('must create account', async () => {
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
            undefined,
            outputStateTreeContext2,
        );

        const acc = await rpc.getCompressedAccount(bn(address.toBuffer()));
        expect(acc!.merkleTree).toEqual(outputStateTreeContext2.tree);
        expect(acc!.queue).toEqual(outputStateTreeContext2.queue!);
    });

    it('getValidityProof [noforester] (inclusion) should return correct trees and queues', async () => {
        // tree 2
        const acc = await rpc.getCompressedAccount(bn(address.toBuffer()));

        const hash = bn(acc!.hash);
        expect(acc?.merkleTree).toEqual(outputStateTreeContext2.tree);
        expect(acc?.queue).toEqual(outputStateTreeContext2.queue!);

        const validityProof = await rpc.getValidityProof([hash]);

        expect(validityProof.merkleTrees[0]).toEqual(
            outputStateTreeContext2.tree,
        );
        expect(validityProof.queues[0]).toEqual(outputStateTreeContext2.queue!);

        await transfer(
            rpc,
            payer,
            1e5,
            payer,
            bob.publicKey,
            outputStateTreeContext,
        );
        executedTxs++;

        const tree2 = pickRandomTreeAndQueue(
            await rpc.getCachedActiveStateTreeInfo(),
        );
        await transfer(
            rpc,
            payer,
            1e5,
            payer,
            bob.publicKey,
            outputStateTreeContext,
        );
        executedTxs++;

        const validityProof2 = await rpc.getValidityProof([hash]);

        expect(validityProof2.merkleTrees[0]).toEqual(
            outputStateTreeContext2.tree,
        );
        expect(validityProof2.queues[0]).toEqual(
            outputStateTreeContext2.queue!,
        );
    });

    it('getValidityProof [noforester] (combined) should return correct trees and queues', async () => {
        const senderAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        const hash = bn(senderAccounts.items[0].hash);

        const newAddressSeeds = [
            new Uint8Array([
                1, 2, 3, 4, 5, 6, 7, 20, 21, 22, 42, 30, 40, 10, 13, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 32, 32, 27, 28, 29, 30, 31, 32,
            ]),
        ];
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
            validityProof.merkleTrees[0].equals(
                senderAccounts.items[0].merkleTree,
            ),
            'Mismatch in merkleTrees expected: ' +
                senderAccounts.items[0].merkleTree +
                ' got: ' +
                validityProof.merkleTrees[0],
        );
        assert.isTrue(
            validityProof.queues[0].equals(senderAccounts.items[0].queue),
            `Mismatch in queues expected: ${senderAccounts.items[0].queue} got: ${validityProof.queues[0]}`,
        );

        await createAccountWithLamports(
            rpc,
            payer,
            [new Uint8Array(randomBytes(32))],
            0,
            LightSystemProgram.programId,
            undefined,
            undefined,
            outputStateTreeContext,
        );
        executedTxs++;
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
                assert.isTrue(
                    proof.merkleTree.equals(
                        prePayerAccounts.items[index].merkleTree,
                    ),
                    `Iteration ${round + 1}: Mismatch in merkleTree for account index ${index}`,
                );
                assert.isTrue(
                    proof.queue.equals(prePayerAccounts.items[index].queue),
                    `Iteration ${round + 1}: Mismatch in queue for account index ${index}`,
                );
            });

            await transfer(
                rpc,
                payer,
                transferAmount,
                payer,
                bob.publicKey,
                outputStateTreeContext,
            );
            executedTxs++;
        }
    });

    it('getMultipleCompressedAccounts should match', async () => {
        await compress(
            rpc,
            payer,
            1e9,
            payer.publicKey,
            outputStateTreeContext2,
        );
        executedTxs++;

        const senderAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        const compressedAccounts = await rpc.getMultipleCompressedAccounts(
            senderAccounts.items.map(account => bn(account.hash)),
        );

        compressedAccounts.forEach((account, index) => {
            assert.isTrue(
                account.merkleTree.equals(
                    senderAccounts.items[index].merkleTree,
                ),
                `Mismatch in merkleTree for account index ${index}`,
            );
            assert.isTrue(
                account.queue.equals(senderAccounts.items[index].queue),
                `Mismatch in queue for account index ${index}`,
            );
        });
    });
});

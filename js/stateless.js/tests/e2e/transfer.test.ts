import { describe, it, assert, beforeAll } from 'vitest';
import { Keypair, Signer } from '@solana/web3.js';
import { newAccountWithLamports } from '../../src/utils/test-utils';
import { Rpc, createRpc } from '../../src/rpc';
import { bn, compress } from '../../src';
import { transfer } from '../../src/actions/transfer';

describe('transfer', () => {
    let rpc: Rpc;
    let payer: Signer;
    let bob: Signer;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 2e9);
        bob = await newAccountWithLamports(rpc, 2e9); // TODO: rand

        await compress(rpc, payer, 1e9, payer.publicKey);
    });

    const numberOfTransfers = 15;
    it(`should send compressed lamports alice -> bob for ${numberOfTransfers} transfers in a loop`, async () => {
        const transferAmount = 1000;
        for (let i = 0; i < numberOfTransfers; i++) {
            console.log('iteration: ', i + 1);
            const preSenderBalance = (
                await rpc.getCompressedAccountsByOwner(payer.publicKey)
            ).reduce((acc, account) => acc.add(account.lamports), bn(0));

            const preReceiverBalance = (
                await rpc.getCompressedAccountsByOwner(bob.publicKey)
            ).reduce((acc, account) => acc.add(account.lamports), bn(0));

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
                postSenderBalance.sub(preSenderBalance).eq(bn(0)),
                `Iteration ${i + 1}: Sender balance should decrease by ${transferAmount}`,
            );
            assert(
                postReceiverBalance.sub(preReceiverBalance).eq(bn(0)),
                `Iteration ${i + 1}: Receiver balance should increase by ${transferAmount}`,
            );
        }
    });
});

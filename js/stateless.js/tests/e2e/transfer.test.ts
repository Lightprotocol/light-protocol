import { describe, it, assert, beforeAll } from 'vitest';
import {  Keypair, LAMPORTS_PER_SOL, Signer } from '@solana/web3.js';
import { createRpc, Rpc } from '../../src/rpc';
import { airdropSol, bn, compress } from '../../src';
import { transfer } from '../../src/actions/transfer';

const LAMPORTS = 1e9;

// Keypairs (consider moving these to a separate config file)
const bobKeypair = new Uint8Array([
    46, 239, 29, 58, 196, 181, 39, 77, 196, 54, 249, 108, 80, 144, 32, 168, 245, 161, 146, 92, 180, 79,
    231, 37, 50, 88, 220, 48, 9, 146, 249, 82, 130, 60, 106, 251, 24, 224, 192, 108, 70, 59, 111, 251,
    186, 50, 23, 103, 106, 233, 113, 148, 57, 190, 158, 111, 163, 28, 157, 47, 201, 41, 249, 59
  ]);
  
  const payerKeypair = new Uint8Array([
    125, 14, 244, 185, 193, 42, 156, 191, 212, 42, 239, 56, 169, 240, 239, 52, 95, 215, 240, 86, 151, 212,
    245, 230, 198, 148, 12, 230, 83, 57, 56, 244, 191, 129, 151, 233, 233, 129, 21, 255, 101, 163, 48, 212,
    218, 82, 134, 36, 29, 185, 30, 215, 183, 242, 244, 222, 8, 10, 158, 214, 99, 237, 126, 9
  ]);


async function airdropIfNeeded(payer: Keypair, bob: Keypair, rpc: Rpc): Promise<void> {
    console.log("Payer:", payer.publicKey.toBase58());
    console.log("Bob:", bob.publicKey.toBase58());
  
    const [bobBalance, payerBalance] = await Promise.all([
      rpc.getBalance(bob.publicKey),
      rpc.getBalance(payer.publicKey)
    ]);
  
    console.log('Bob balance:', bobBalance);
    console.log('Payer balance:', payerBalance);
  
    if (payerBalance < LAMPORTS_PER_SOL) {
      console.log('Airdropping SOL to payer');
      await airdropSol({ connection: rpc, lamports: LAMPORTS, recipientPublicKey: payer.publicKey });
      console.log('New payer balance:', await rpc.getBalance(payer.publicKey));
    }
  
    if (bobBalance < LAMPORTS_PER_SOL) {
      console.log('Airdropping SOL to Bob');
      await airdropSol({ connection: rpc, lamports: LAMPORTS, recipientPublicKey: bob.publicKey });
      console.log('New Bob balance:', await rpc.getBalance(bob.publicKey));
    }
  }
  
  
describe('transfer', () => {
    let rpc: Rpc;
    let payer: Signer;
    let bob: Signer;

    function devnetRpc(): Rpc {
        const apiKey = '<HELIUS_API_KEY>';
        return createRpc(
          `https://devnet.helius-rpc.com/?api-key=${apiKey}`,
          `https://devnet.helius-rpc.com/?api-key=${apiKey}`,
          `https://devnet.helius-rpc.com:3001/?api-key=${apiKey}`
        );
    }

    beforeAll(async () => {
        // const lightWasm = await WasmFactory.getInstance();
        //rpc = await getTestRpc(lightWasm);
        rpc = devnetRpc();
        // payer = await newAccountWithLamports(rpc, 2e9, 256);
        // bob = await newAccountWithLamports(rpc, 2e9, 256);
        payer = Keypair.fromSecretKey(payerKeypair);
        bob = Keypair.fromSecretKey(bobKeypair);
        
        // await airdropIfNeeded(payer, bob, rpc);

        // await compress(rpc, payer, 1e7, payer.publicKey);
    });

    const numberOfTransfers = 1;
    it(`should send compressed lamports alice -> bob for ${numberOfTransfers} transfers in a loop`, async () => {
        const transferAmount = 10;
        for (let i = 0; i < numberOfTransfers; i++) {
            // const preSenderBalance = (
            //     await rpc.getCompressedAccountsByOwner(payer.publicKey)
            // ).reduce((acc, account) => acc.add(account.lamports), bn(0));

            // const preReceiverBalance = (
            //     await rpc.getCompressedAccountsByOwner(bob.publicKey)
            // ).reduce((acc, account) => acc.add(account.lamports), bn(0));

            await transfer(rpc, payer, transferAmount, payer, bob.publicKey);

            // const postSenderAccs = await rpc.getCompressedAccountsByOwner(
            //     payer.publicKey,
            // );
            // const postReceiverAccs = await rpc.getCompressedAccountsByOwner(
            //     bob.publicKey,
            // );

            // const postSenderBalance = postSenderAccs.reduce(
            //     (acc, account) => acc.add(account.lamports),
            //     bn(0),
            // );
            // const postReceiverBalance = postReceiverAccs.reduce(
            //     (acc, account) => acc.add(account.lamports),
            //     bn(0),
            // );

            // assert(
            //     postSenderBalance.sub(preSenderBalance).eq(bn(-transferAmount)),
            //     `Iteration ${i + 1}: Sender balance should decrease by ${transferAmount}`,
            // );
            // assert(
            //     postReceiverBalance
            //         .sub(preReceiverBalance)
            //         .eq(bn(transferAmount)),
            //     `Iteration ${i + 1}: Receiver balance should increase by ${transferAmount}`,
            // );
        }
    });
});

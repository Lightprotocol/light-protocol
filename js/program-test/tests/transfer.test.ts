import { describe, it, assert, beforeAll } from "vitest";
import { Keypair } from "@solana/web3.js";
import {
  createLiteSVMRpc,
  newAccountWithLamports,
  LiteSVMRpc,
  NobleHasherFactory,
} from "../src";
import { bn, compress, transfer } from "@lightprotocol/stateless.js";

describe("transfer", () => {
  let rpc: LiteSVMRpc;
  let payer: Keypair;
  let bob: Keypair;

  beforeAll(async () => {
    const lightWasm = await NobleHasherFactory.getInstance();
    rpc = await createLiteSVMRpc(lightWasm);
    payer = await newAccountWithLamports(rpc, 2e9);
    bob = await newAccountWithLamports(rpc, 2e9);

    await compress(rpc, payer, 1e9, payer.publicKey);
  });

  const numberOfTransfers = 10;
  it(`should send compressed lamports alice -> bob for ${numberOfTransfers} transfers in a loop`, async () => {
    const transferAmount = 1000;
    for (let i = 0; i < numberOfTransfers; i++) {
      const preSenderBalance = (
        await rpc.getCompressedAccountsByOwner(payer.publicKey)
      ).items.reduce((acc, account) => acc.add(account.lamports), bn(0));

      const preReceiverBalance = (
        await rpc.getCompressedAccountsByOwner(bob.publicKey)
      ).items.reduce((acc, account) => acc.add(account.lamports), bn(0));

      await transfer(rpc, payer, transferAmount, payer, bob.publicKey);

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
        postSenderBalance.sub(preSenderBalance).eq(bn(-transferAmount)),
        `Iteration ${i + 1}: Sender balance should decrease by ${transferAmount}`,
      );
      assert(
        postReceiverBalance.sub(preReceiverBalance).eq(bn(transferAmount)),
        `Iteration ${i + 1}: Receiver balance should increase by ${transferAmount}`,
      );
    }
  });
});

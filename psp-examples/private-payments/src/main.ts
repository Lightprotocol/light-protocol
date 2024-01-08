import * as light from "@lightprotocol/zk.js";
import * as anchor from "@coral-xyz/anchor";
import { airdropSol, confirmConfig, TestRpc, User } from "@lightprotocol/zk.js";
import { BN } from "@coral-xyz/anchor";

process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
const provider = anchor.AnchorProvider.local(
  "http://127.0.0.1:8899",
  confirmConfig
);

const log = console.log;

const main = async () => {
  const PARTICIPANTS_COUNT = 2;
  const recipients = new Array(PARTICIPANTS_COUNT).fill(null).map(() => {
    return {
      keypair: anchor.web3.Keypair.generate(),
    };
  });

  const senders = new Array(PARTICIPANTS_COUNT).fill(null).map(() => {
    return {
      keypair: anchor.web3.Keypair.generate(),
    };
  });

  const logLabel = `async private payments for ${PARTICIPANTS_COUNT} recipients`;
  console.time(logLabel);
  let calls = [];

  for (let i = 0; i < PARTICIPANTS_COUNT; i++) {
    const sender = senders[i];
    const recipient = recipients[i];
    calls.push(makeShield(sender.keypair, recipient.keypair));
  }
  await Promise.all(calls);
  const lightProvider = await light.Provider.init({
    wallet: senders[0].keypair,
    confirmConfig,
  });
  const rpc = new TestRpc({
    rpcPubkey: senders[0].keypair.publicKey,
    rpcRecipientSol: senders[0].keypair.publicKey,
    rpcFee: new BN(100_000),
    payer: senders[0].keypair,
    connection: provider.connection,
    lightWasm: lightProvider.lightWasm,
  });
  lightProvider.rpc = rpc;
  log("initializing light provider...");

  calls = [];
  for (let i = 0; i < PARTICIPANTS_COUNT; i++) {
    const sender = senders[i];
    const recipient = recipients[i];
    calls.push(makeTransfer(sender.keypair, recipient.keypair));
  }
  await Promise.all(calls);

  console.timeEnd(logLabel);

  async function makeShield(
    sender: anchor.web3.Keypair,
    recipient: anchor.web3.Keypair
  ) {
    log("requesting airdrop...");
    await airdropSol({
      connection: provider.connection,
      lamports: 1e12,
      recipientPublicKey: sender.publicKey,
    });

    log("initializing Solana wallet...");

    log("initializing light provider...");
    const lightProvider = await light.Provider.init({
      wallet: sender,
      confirmConfig,
    });
    log("setting-up test rpc...");
    const rpc = new TestRpc({
      rpcPubkey: sender.publicKey,
      rpcRecipientSol: sender.publicKey,
      rpcFee: new BN(100_000),
      payer: sender,
      connection: provider.connection,
      lightWasm: lightProvider.lightWasm,
    });
    lightProvider.rpc = rpc;

    log("initializing user...");
    const user = await light.User.init({ provider: lightProvider });

    try {
      await user.shield({
        publicAmountSol: "1",
        token: "SOL",
        confirmOptions: light.ConfirmOptions.finalized,
      });
    } catch (e) {}
  }

  async function makeTransfer(
    sender: anchor.web3.Keypair,
    recipient: anchor.web3.Keypair
  ) {
    log("initializing light provider...");
    const lightProvider = await light.Provider.init({
      wallet: sender,
      confirmConfig,
    });
    log("initializing Solana wallet...");
    log("setting-up test rpc...");
    const rpc = new TestRpc({
      rpcPubkey: sender.publicKey,
      rpcRecipientSol: sender.publicKey,
      rpcFee: new BN(100_000),
      payer: sender,
      connection: provider.connection,
      lightWasm: lightProvider.lightWasm,
    });
    lightProvider.rpc = rpc;

    log("initializing user...");
    const user = await light.User.init({ provider: lightProvider });

    log("getting user balance...");
    log(await user.getBalance());

    log("requesting airdrop...");
    await airdropSol({
      connection: provider.connection,
      lamports: 2e9,
      recipientPublicKey: recipient.publicKey,
    });

    log("initializing light provider recipient...");
    const lightProviderRecipient = await light.Provider.init({
      wallet: recipient,
      rpc,
      confirmConfig,
    });

    log("initializing light user recipient...");
    const testRecipient: User = await light.User.init({
      provider: lightProviderRecipient,
    });

    log("executing transfer...");
    try {
      const response = await user.transfer({
        amountSol: "0.25",
        token: "SOL",
        recipient: testRecipient.account.getPublicKey(),
        confirmOptions: light.ConfirmOptions.finalized,
      });
      log("getting tx hash...");
      log(response.txHash);
    } catch (e) {}
    log("getting UTXO inbox...");
    log(await testRecipient.getUtxoInbox());
  }
};

log("running program...");
main()
  .then(() => {
    log("running complete.");
  })
  .catch((e) => {
    console.trace(e);
  })
  .finally(() => {
    log("exiting program.");
    process.exit(0);
  });

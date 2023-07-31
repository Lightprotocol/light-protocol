import * as anchor from "@coral-xyz/anchor";
import { PublicKey, Keypair as SolanaKeypair } from "@solana/web3.js";
let circomlibjs = require("circomlibjs");

import {
  initLookUpTableFromFile,
  ADMIN_AUTH_KEYPAIR,
  createTestAccounts,
  confirmConfig,
  TestRelayer,
  Action,
  LOOK_UP_TABLE,
<<<<<<< HEAD
=======
  Provider,
  User,
  MINT,
  airdropShieldedSol,
  airdropShieldedMINTSpl,
>>>>>>> main
} from "@lightprotocol/zk.js";

import {
  performShielding,
  EnvironmentConfig,
  performMergeAll,
  performMergeUtxos,
} from "./test-utils/user-utils";

import { BN } from "@coral-xyz/anchor";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

const recipientSeed = bs58.encode(new Uint8Array(32).fill(7));

describe("Test User merge 1 sol utxo and one spl utxo in sequence ", () => {
  // Configure the client to use the local cluster.
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

  const anchorProvider = anchor.AnchorProvider.local(
    "http://127.0.0.1:8899",
    confirmConfig,
  );
  anchor.setProvider(anchorProvider);

  const userKeypair = ADMIN_AUTH_KEYPAIR;
  var environmentConfig: EnvironmentConfig = {};

  before("init test setup Merkle tree lookup table etc ", async () => {
    await createTestAccounts(anchorProvider.connection);
    environmentConfig.lookUpTable = await initLookUpTableFromFile(
      anchorProvider,
    );

    environmentConfig.poseidon = await circomlibjs.buildPoseidonOpt();
    // this keypair is used to derive the shielded account seed from the light message signature
    environmentConfig.providerSolanaKeypair = ADMIN_AUTH_KEYPAIR;
    const relayerRecipientSol = SolanaKeypair.generate().publicKey;

    await anchorProvider.connection.requestAirdrop(
      relayerRecipientSol,
      2_000_000_000,
    );

    environmentConfig.relayer = new TestRelayer({
      relayerPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
      relayerRecipientSol,
      relayerFee: new anchor.BN(100_000),
      payer: ADMIN_AUTH_KEYPAIR,
    });
  });

  it("Merge all sol & spl (no existing utxo)", async () => {
    let testInputsShieldSol = {
      amountSpl: 0,
      amountSol: 15,
      token: "SOL",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
      recipientAccount: userKeypair,
      mergedUtxo: false,
      shieldToRecipient: true,
      recipientSeed,
    };

    await performShielding({
      numberOfShields: 1,
      testInputs: testInputsShieldSol,
      environmentConfig,
    });

    let testInputsShieldSpl = {
      amountSpl: 20,
      token: "USDC",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      shieldToRecipient: true,
      recipientSeed,
    };

    await performShielding({
      numberOfShields: 1,
      testInputs: testInputsShieldSpl,
      environmentConfig,
    });

    let testInputsMergeAllSol = {
      token: "SOL",
      type: Action.TRANSFER,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      recipientSeed,
    };

    await performMergeAll({
      environmentConfig,
      testInputs: testInputsMergeAllSol,
    });

    let testInputsMergeAllSpl = {
      type: Action.TRANSFER,
      token: "USDC",
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      recipientSeed,
    };

    await performMergeAll({
      environmentConfig,
      testInputs: testInputsMergeAllSpl,
    });
  });

  it("Merge all spl (existing utxos)", async () => {
    let testInputsShield = {
      amountSpl: 20,
      token: "USDC",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      shieldToRecipient: true,
      recipientSeed,
    };

    await performShielding({
      numberOfShields: 2,
      testInputs: testInputsShield,
      environmentConfig,
    });

    let testInputs = {
      type: Action.TRANSFER,
      token: "USDC",
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      recipientSeed,
    };
    await performMergeAll({
      environmentConfig,
      testInputs,
    });
  });

  it("Merge one spl (existing utxos)", async () => {
    await airdropShieldedMINTSpl({
      seed: recipientSeed,
      amount: 11,
    });
    // shield SPL to recipient
    let testInputsShield = {
      amountSpl: 20,
      token: "USDC",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      shieldToRecipient: true,
      recipientSeed,
    };

    await performShielding({
      numberOfShields: 2,
      testInputs: testInputsShield,
      environmentConfig,
    });
    const provider = await Provider.init({
      wallet: userKeypair,
      relayer: environmentConfig.relayer,
    });

    const userSender: User = await User.init({
      provider,
      seed: recipientSeed,
    });

    const utxoCommitmment: string = (
      await userSender.getUtxoInbox()
    ).tokenBalances
      .get(MINT.toBase58())
      .utxos.keys()
      .next().value;

    let testInputs = {
      type: Action.TRANSFER,
      token: "USDC",
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      utxoCommitments: [utxoCommitmment],
      recipientSeed,
    };

    await performMergeUtxos({
      testInputs,
      environmentConfig,
    });
  });

  it("Merge all sol (existing utxos)", async () => {
    await airdropShieldedSol({
      seed: recipientSeed,
      amount: 15,
    });
    let testInputsShield = {
      amountSol: 20,
      token: "SOL",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      shieldToRecipient: true,
      recipientSeed,
    };

    await performShielding({
      numberOfShields: 2,
      testInputs: testInputsShield,
      environmentConfig,
    });

    let testInputs = {
      type: Action.TRANSFER,
      token: "SOL",
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      recipientSeed,
    };

    await performMergeAll({
      environmentConfig,
      testInputs,
    });
  });

  it("Merge one sol (existing utxos)", async () => {
    await airdropShieldedSol({
      seed: recipientSeed,
      amount: 1,
    });
    let testInputsShield = {
      amountSpl: 0,
      amountSol: 20,
      token: "SOL",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      shieldToRecipient: true,
      recipientSeed,
    };

    await performShielding({
      numberOfShields: 2,
      testInputs: testInputsShield,
      environmentConfig,
    });

    const provider = await Provider.init({
      wallet: userKeypair,
      relayer: environmentConfig.relayer,
    });

    const userSender: User = await User.init({
      provider,
      seed: recipientSeed,
    });

    const utxoCommitmment: string = (
      await userSender.getUtxoInbox()
    ).tokenBalances
      .get(PublicKey.default.toBase58())
      .utxos.keys()
      .next().value;

    let testInputs = {
      type: Action.TRANSFER,
      token: "SOL",
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      utxoCommitments: [utxoCommitmment],
      recipientSeed,
    };

    await performMergeUtxos({
      testInputs,
      environmentConfig,
    });
  });
});

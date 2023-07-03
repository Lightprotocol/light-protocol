import * as anchor from "@coral-xyz/anchor";
import { Keypair as SolanaKeypair, SystemProgram } from "@solana/web3.js";
let circomlibjs = require("circomlibjs");

import {
  setUpMerkleTree,
  initLookUpTableFromFile,
  ADMIN_AUTH_KEYPAIR,
  createTestAccounts,
  confirmConfig,
  TestRelayer,
  Action,
} from "@lightprotocol/zk.js";

import {
  performShielding,
  EnvironmentConfig,
  performMergeAll,
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

    environmentConfig.relayer = new TestRelayer(
      userKeypair.publicKey,
      environmentConfig.lookUpTable,
      relayerRecipientSol,
      new BN(100000),
      new BN(10_100_000),
      userKeypair,
    );
  });

  it("(user class) shield SOL to recipient", async () => {
    let testInputs = {
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
      testInputs,
      environmentConfig,
    });
  });

  it("(user class) shield SPL to recipient", async () => {
    let testInputs = {
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
      testInputs,
      environmentConfig,
    });
  });

  it("(user class) merge all sol (no existing utxo)", async () => {
    let testInputs = {
      token: "SOL",
      type: Action.TRANSFER,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      recipientSeed,
    };

    await performMergeAll({
      environmentConfig,
      testInputs,
    });
  });

  it("(user class) merge all spl (no existing utxo)", async () => {
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
});

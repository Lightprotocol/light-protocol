import * as anchor from "@coral-xyz/anchor";
import { Keypair as SolanaKeypair, SystemProgram } from "@solana/web3.js";
let circomlibjs = require("circomlibjs");

import {
  setUpMerkleTree,
  initLookUpTableFromFile,
  ADMIN_AUTH_KEYPAIR,
  Provider,
  createTestAccounts,
  confirmConfig,
  User,
  Account,
  TestRelayer,
  Action,
  TestStateValidator,
  airdropShieldedSol,
  airdropSol,
} from "@lightprotocol/zk.js";

import { BN } from "@coral-xyz/anchor";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import {
  EnvironmentConfig,
  performMergeAll,
  performShielding,
} from "./test-utils/user-utils";

var LOOK_UP_TABLE;
var POSEIDON;
var RELAYER: TestRelayer, provider: Provider;
const recipientSeed = bs58.encode(new Uint8Array(32).fill(7));

// TODO: remove deprecated function calls
describe("Test User", () => {
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

    await airdropSol({
      provider: anchorProvider,
      amount: 1_000_000_000,
      recipientPublicKey: relayerRecipientSol,
    });

    environmentConfig.relayer = new TestRelayer(
      userKeypair.publicKey,
      environmentConfig.lookUpTable,
      relayerRecipientSol,
      new BN(100000),
    );

    await airdropShieldedSol({
      seed: recipientSeed,
      amount: 15,
    });
  });

  it("(user class) shield SPL to recipient", async () => {
    let testInputs = {
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
      testInputs,
      environmentConfig,
    });
  });

  it("(user class) merge all spl (one existing utxo)", async () => {
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
});

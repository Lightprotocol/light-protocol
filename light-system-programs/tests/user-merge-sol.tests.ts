import * as anchor from "@coral-xyz/anchor";
import { Keypair as SolanaKeypair } from "@solana/web3.js";
let circomlibjs = require("circomlibjs");

import {
  initLookUpTableFromFile,
  ADMIN_AUTH_KEYPAIR,
  createTestAccounts,
  confirmConfig,
  TestRelayer,
  Action,
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
      lamports: 1_000_000_000,
      recipientPublicKey: relayerRecipientSol,
    });

    environmentConfig.relayer = new TestRelayer({
      relayerPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
      lookUpTable: LOOK_UP_TABLE,
      relayerRecipientSol,
      relayerFee: new anchor.BN(100_000),
      payer: ADMIN_AUTH_KEYPAIR,
    });

    await airdropShieldedSol({
      seed: recipientSeed,
      amount: 15,
    });
  });

  it("(user class) shield SPL to recipient", async () => {
    let testInputs = {
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

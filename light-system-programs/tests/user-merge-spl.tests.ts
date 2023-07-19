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
  airdropSol,
  airdropShieldedMINTSpl,
  LOOK_UP_TABLE,
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

    environmentConfig.relayer = await new TestRelayer({
      relayerPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
      lookUpTable: LOOK_UP_TABLE,
      relayerRecipientSol: relayerRecipientSol,
      relayerFee: new BN(100_000),
      payer: ADMIN_AUTH_KEYPAIR,
    });

    await airdropShieldedMINTSpl({
      seed: recipientSeed,
      amount: 11,
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
      numberOfShields: 2,
      testInputs,
      environmentConfig,
    });
  });

  it("(user class) merge all spl (one existing utxo)", async () => {
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

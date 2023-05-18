import * as anchor from "@coral-xyz/anchor";
import { Keypair as SolanaKeypair } from "@solana/web3.js";
let circomlibjs = require("circomlibjs");

import {
  setUpMerkleTree,
  initLookUpTableFromFile,
  ADMIN_AUTH_KEYPAIR,
  createTestAccounts,
  confirmConfig,
  TestRelayer,
  Action,
  Provider,
  User,
  MINT,
  airdropShieldedSol,
  airdropSol,
} from "@lightprotocol/zk.js";

import { BN } from "@coral-xyz/anchor";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import {
  EnvironmentConfig,
  performMergeUtxos,
  performShielding,
} from "./test-utils/user-utils";

const recipientSeed = bs58.encode(new Uint8Array(32).fill(7));

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
      amount: 1_000_000,
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
      amount: 1,
    });
  });

  it("(user class) shield SPL to recipient", async () => {
    let testInputs = {
      amountSpl: 20,
      amountSol: 0,
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

  it("(user class) merge one spl (one existing utxo)", async () => {
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
});

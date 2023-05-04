import * as anchor from "@coral-xyz/anchor";
import {
  Keypair,
  Keypair as SolanaKeypair,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import _ from "lodash";
import { assert } from "chai";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// init chai-as-promised support
chai.use(chaiAsPromised);

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
  MINT,
} from "light-sdk";

import { BN } from "@coral-xyz/anchor";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

var LOOK_UP_TABLE;
var POSEIDON;
var RELAYER: TestRelayer, provider: Provider;
const senderAccountSeed = bs58.encode(new Uint8Array(32).fill(7));

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
  let seed32 = bs58.encode(new Uint8Array(32).fill(1));

  const userKeypair = ADMIN_AUTH_KEYPAIR;
  let utxoCommitmentArray: string[] = [];
  before("init test setup Merkle tree lookup table etc ", async () => {
    await createTestAccounts(anchorProvider.connection);
    LOOK_UP_TABLE = await initLookUpTableFromFile(anchorProvider);
    await setUpMerkleTree(anchorProvider);
    POSEIDON = await circomlibjs.buildPoseidonOpt();

    const relayerRecipientSol = SolanaKeypair.generate().publicKey;

    await anchorProvider.connection.requestAirdrop(
      relayerRecipientSol,
      2_000_000_000,
    );

    RELAYER = await new TestRelayer(
      userKeypair.publicKey,
      LOOK_UP_TABLE,
      relayerRecipientSol,
      new BN(100000),
    );
    provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    });
  });

  it("(user class) shield SPL", async () => {
    provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    });
    for (var i = 0; i < 1; i++) {
      let testInputs = {
        amountSpl: 11,
        amountSol: 0,
        token: "USDC",
        type: Action.SHIELD,
        expectedUtxoHistoryLength: 1,
        expectedSpentUtxosLength: 0,
      };

      const userSender: User = await User.init({
        provider,
        seed: senderAccountSeed,
      });

      const testStateValidator = new TestStateValidator({
        userSender,
        userRecipient: userSender,
        provider,
        testInputs,
      });

      await testStateValidator.fetchAndSaveState();

      await userSender.shield({
        publicAmountSpl: testInputs.amountSpl,
        token: testInputs.token,
      });

      await userSender.provider.latestMerkleTree();

      await testStateValidator.checkTokenShielded();
    }
  });

  it("(user class) shield SPL to recipient", async () => {
    provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    });
    for (var i = 0; i < 2; i++) {
      let testInputs = {
        amountSpl: 20,
        amountSol: 0,
        token: "USDC",
        type: Action.SHIELD,
        expectedUtxoHistoryLength: 1,
        expectedSpentUtxosLength: 0,
        shieldToRecipient: true,
      };

      const userSender: User = await User.init({
        provider,
      });

      const userRecipient: User = await User.init({
        provider,
        seed: senderAccountSeed,
      });

      const testStateValidator = new TestStateValidator({
        userSender,
        userRecipient,
        provider,
        testInputs,
      });

      await testStateValidator.fetchAndSaveState();

      await userSender.shield({
        publicAmountSpl: testInputs.amountSpl,
        token: testInputs.token,
        recipient: userRecipient.account.getPublicKey(),
      });

      await userSender.provider.latestMerkleTree();

      await testStateValidator.checkTokenShielded();
    }
  });

  it("(user class) merge one spl (one existing utxo)", async () => {
    const provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    });

    const userSender: User = await User.init({
      provider,
      seed: senderAccountSeed,
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
      shieldToRecipient: true,
      utxoCommitments: [utxoCommitmment],
    };

    const testStateValidator = new TestStateValidator({
      userSender,
      userRecipient: userSender,
      provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();

    await userSender.mergeUtxos([utxoCommitmment], MINT);

    /**
     * Test:
     * - if user utxo were less than 10 before, there is only one balance utxo of asset all others have been merged
     * - min(10 - nrPreBalanceUtxos[asset], nrPreBalanceInboxUtxos[asset]) have been merged thus size of utxos is less by that number
     * -
     */
    // TODO: add random amount and amount checks
    await userSender.provider.latestMerkleTree();
    await testStateValidator.checkMerged();
  });
});

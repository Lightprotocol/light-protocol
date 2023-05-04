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

describe("Test User merge 1 sol utxo and one spl utxo in sequence ", () => {
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

  it("(user class) shield SOL to recipient", async () => {
    for (var i = 0; i < 1; i++) {
      let testInputs = {
        amountSpl: 0,
        amountSol: 15,
        token: "SOL",
        type: Action.SHIELD,
        expectedUtxoHistoryLength: i + 1,
        recipientAccount: userKeypair,
        mergedUtxo: false,
        shieldToRecipient: true,
      };

      const provider = await Provider.init({
        wallet: userKeypair,
        relayer: RELAYER,
      });

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

      let recipient = Account.fromPubkey(
        userRecipient.account.getPublicKey(),
        POSEIDON,
      );
      await userSender.shield({
        publicAmountSol: testInputs.amountSol,
        token: testInputs.token,
        recipient,
      });

      await userRecipient.provider.latestMerkleTree();
      await testStateValidator.checkSolShielded();
    }
  });

  it("(user class) shield SPL to recipient", async () => {
    for (var i = 0; i < 1; i++) {
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
        recipient: Account.fromPubkey(
          userRecipient.account.getPublicKey(),
          POSEIDON,
        ),
      });

      await userSender.provider.latestMerkleTree();

      await testStateValidator.checkTokenShielded();
    }
  });

  it("(user class) merge all sol (no existing utxo)", async () => {
    let testInputs = {
      token: "SOL",
      type: Action.TRANSFER,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      shieldToRecipient: true,
    };

    const provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    });

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
    await userSender.mergeAllUtxos(SystemProgram.programId);

    /**
     * Test:
     * - if user utxo were less than 10 before, there is only one balance utxo of asset all others have been merged
     * - min(10 - nrPreBalanceUtxos[asset], nrPreBalanceInboxUtxos[asset]) have been merged thus size of utxos is less by that number
     * -
     */
    // TODO: add random amount and amount checks
    await userSender.provider.latestMerkleTree();
    await testStateValidator.checkMergedAll();
  });
  it("(user class) merge all spl (no existing utxo)", async () => {
    let testInputs = {
      type: Action.TRANSFER,
      token: "USDC",
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      shieldToRecipient: true,
    };

    const provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    });

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
    await userSender.mergeAllUtxos(MINT);

    /**
     * Test:
     * - if user utxo were less than 10 before, there is only one balance utxo of asset all others have been merged
     * - min(10 - nrPreBalanceUtxos[asset], nrPreBalanceInboxUtxos[asset]) have been merged thus size of utxos is less by that number
     * -
     */
    // TODO: add random amount and amount checks
    await userSender.provider.latestMerkleTree();
    await testStateValidator.checkMergedAll();
  });
});

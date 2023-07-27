import * as anchor from "@coral-xyz/anchor";
import { Keypair, Keypair as SolanaKeypair } from "@solana/web3.js";
import _ from "lodash";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// init chai-as-promised support
chai.use(chaiAsPromised);
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
let circomlibjs = require("circomlibjs");
import {
  ADMIN_AUTH_KEYPAIR,
  Provider,
  createTestAccounts,
  confirmConfig,
  User,
  Account,
  CreateUtxoErrorCode,
  UserErrorCode,
  TransactionErrorCode,
  ADMIN_AUTH_KEY,
  TestRelayer,
  Action,
  UserTestAssertHelper,
  generateRandomTestAmount,
  airdropSol,
  ConfirmOptions,
  airdropShieldedSol,
  TOKEN_ACCOUNT_FEE,
} from "@lightprotocol/zk.js";

import { BN } from "@coral-xyz/anchor";
import { assert } from "chai";

var POSEIDON;
var RELAYER: TestRelayer, provider: Provider, user: User;

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

  before("init test setup Merkle tree lookup table etc ", async () => {
    await createTestAccounts(anchorProvider.connection);
    POSEIDON = await circomlibjs.buildPoseidonOpt();

    const relayerRecipientSol = SolanaKeypair.generate().publicKey;
    await anchorProvider.connection.requestAirdrop(
      relayerRecipientSol,
      2_000_000_000,
    );
    let relayer = Keypair.generate();
    await airdropSol({
      connection: anchorProvider.connection,
      lamports: 2_000_000_000,
      recipientPublicKey: relayer.publicKey,
    });
    RELAYER = new TestRelayer({
      relayerPubkey: relayer.publicKey,
      relayerRecipientSol,
      relayerFee: new anchor.BN(100_000),
      highRelayerFee: TOKEN_ACCOUNT_FEE,
      payer: relayer,
    });

    provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
      confirmConfig,
    });
    await airdropSol({
      connection: anchorProvider.connection,
      lamports: 2_000_000_000,
      recipientPublicKey: userKeypair.publicKey,
    });

    user = await User.init({ provider });
  });

  it("(user class) shield SPL", async () => {
    var expectedSpentUtxosLength = 0;
    var expectedUtxoHistoryLength = 1;
    let testInputs = {
      amountSpl: generateRandomTestAmount(0, 100000, 2),
      token: "USDC",
      type: Action.SHIELD,
      expectedUtxoHistoryLength,
      expectedSpentUtxosLength,
    };

    const testStateValidator = new UserTestAssertHelper({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();

    await user.shield({
      publicAmountSpl: testInputs.amountSpl,
      token: testInputs.token,
    });

    await testStateValidator.checkSplShielded();
  });

  it("(user class) shield SOL", async () => {
    let testInputs = {
      amountSol: 15,
      token: "SOL",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
    };

    const testStateValidator = new UserTestAssertHelper({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();

    await user.shield({
      publicAmountSol: testInputs.amountSol,
      token: testInputs.token,
    });

    await testStateValidator.checkSolShielded();
  });

  it("(user class) confirm options SPL", async () => {
    const userSeed = bs58.encode(new Uint8Array(32).fill(3));
    await airdropShieldedSol({ provider, amount: 10, seed: userSeed });
    let testInputs = {
      amountSpl: 15,
      token: "USDC",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
      recipientSeed: userSeed,
    };
    const user: User = await User.init({ provider, seed: userSeed });

    const testStateValidator = new UserTestAssertHelper({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();

    await user.shield({
      publicAmountSpl: testInputs.amountSpl,
      token: testInputs.token,
      confirmOptions: ConfirmOptions.finalized,
    });

    await testStateValidator.checkCommittedBalanceSpl();

    const recipientSeed = bs58.encode(new Uint8Array(32).fill(8));
    const recipientUser: User = await User.init({
      provider,
      seed: recipientSeed,
    });

    let testInputsTransfer = {
      amountSpl: 1,
      token: "USDC",
      type: Action.TRANSFER,
      expectedUtxoHistoryLength: 1,
      recipientSeed,
    };

    const testStateValidatorTransfer = new UserTestAssertHelper({
      userSender: user,
      userRecipient: recipientUser,
      provider,
      testInputs: testInputsTransfer,
    });
    await testStateValidatorTransfer.fetchAndSaveState();

    await user.getBalance();
    await user.transfer({
      amountSpl: testInputsTransfer.amountSpl,
      token: testInputsTransfer.token,
      confirmOptions: ConfirmOptions.finalized,
      recipient: recipientUser.account.getPublicKey(),
    });

    await testStateValidatorTransfer.checkCommittedBalanceSpl();

    let testInputsUnshield = {
      amountSpl: 0.5,
      token: "USDC",
      type: Action.UNSHIELD,
      expectedUtxoHistoryLength: 2,
      recipientSeed: userSeed,
    };

    const testStateValidatorUnshield = new UserTestAssertHelper({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs: testInputsUnshield,
    });
    await testStateValidatorUnshield.fetchAndSaveState();

    const recipient = SolanaKeypair.generate();
    await user.getBalance();
    await user.unshield({
      publicAmountSol: testInputsUnshield.amountSpl,
      token: testInputsUnshield.token,
      confirmOptions: ConfirmOptions.finalized,
      recipient: recipient.publicKey,
    });
    let recipientBalance = await provider.provider.connection.getBalance(
      recipient.publicKey,
    );
    assert.equal(recipientBalance, 0.5e9);
    await testStateValidatorUnshield.checkCommittedBalanceSpl();
  });

  it("(user class) unshield SPL", async () => {
    const solRecipient = SolanaKeypair.generate();

    const testInputs = {
      amountSpl: 1,
      token: "USDC",
      type: Action.UNSHIELD,
      recipient: solRecipient.publicKey,
      expectedUtxoHistoryLength: 1,
    };

    const testStateValidator = new UserTestAssertHelper({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();

    await user.unshield({
      publicAmountSpl: testInputs.amountSpl,
      token: testInputs.token,
      recipient: testInputs.recipient,
    });

    await user.provider.latestMerkleTree();

    await testStateValidator.checkSplUnshielded();
  });

  it("(user class) transfer SPL", async () => {
    const testInputs = {
      amountSpl: 1,
      amountSol: 0,
      token: "USDC",
      type: Action.TRANSFER,
      expectedUtxoHistoryLength: 1,
      recipientSeed: bs58.encode(new Uint8Array(32).fill(9)),
      expectedRecipientUtxoLength: 1,
    };

    const recipientAccount = new Account({
      poseidon: POSEIDON,
      seed: testInputs.recipientSeed,
    });

    const userRecipient: User = await User.init({
      provider,
      seed: testInputs.recipientSeed,
    });

    const testStateValidator = new UserTestAssertHelper({
      userSender: user,
      userRecipient,
      provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();

    await user.transfer({
      amountSpl: testInputs.amountSpl,
      token: testInputs.token,
      recipient: recipientAccount.getPublicKey(),
    });

    await user.provider.latestMerkleTree();
    await user.getBalance();
    await testStateValidator.checkSplTransferred();
  });

  it("(user class) storage shield", async () => {
    let testInputs = {
      amountSpl: 0,
      amountSol: 0,
      token: "SOL",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
      storage: true,
      message: Buffer.alloc(512).fill(1),
    };

    const testStateValidator = new UserTestAssertHelper({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();
    await user.storeData(testInputs.message, ConfirmOptions.spendable, true);
    await user.provider.latestMerkleTree();

    await testStateValidator.assertStoredWithShield();
  });

  it("(user class) storage transfer", async () => {
    let testInputs = {
      amountSpl: 0,
      amountSol: 0,
      token: "SOL",
      type: Action.TRANSFER,
      expectedUtxoHistoryLength: 1,
      storage: true,
      isSender: true,
      message: Buffer.alloc(672).fill(2),
    };

    const seed = bs58.encode(new Uint8Array(32).fill(4));
    await airdropShieldedSol({ provider, amount: 1, seed });

    await provider.latestMerkleTree();
    const user: User = await User.init({ provider, seed });

    const testStateValidator = new UserTestAssertHelper({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();

    await user.storeData(testInputs.message, ConfirmOptions.spendable, false);
    await user.provider.latestMerkleTree();
    await testStateValidator.assertStoredWithTransfer();
  });
});

describe("Test User Errors", () => {
  // Configure the client to use the local cluster.
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

  const providerAnchor = anchor.AnchorProvider.local(
    "http://127.0.0.1:8899",
    confirmConfig,
  );
  anchor.setProvider(providerAnchor);

  const userKeypair = ADMIN_AUTH_KEYPAIR;

  let amount, token, provider, user;
  before("init test setup Merkle tree lookup table etc ", async () => {
    if ((await providerAnchor.connection.getBalance(ADMIN_AUTH_KEY)) === 0) {
      await createTestAccounts(providerAnchor.connection);
    }

    POSEIDON = await circomlibjs.buildPoseidonOpt();
    amount = 20;
    token = "USDC";

    provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
      confirmConfig,
    });

    user = await User.init({ provider });
  });
  it("NO_PUBLIC_AMOUNTS_PROVIDED shield", async () => {
    await chai.assert.isRejected(
      user.shield({ token }),
      CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED,
    );
  });

  it("TOKEN_UNDEFINED shield", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.shield({ publicAmountSpl: amount }),
      UserErrorCode.TOKEN_UNDEFINED,
    );
  });

  it("INVALID_TOKEN shield", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.shield({ publicAmountSpl: amount, token: "SOL" }),
      UserErrorCode.INVALID_TOKEN,
    );
  });

  it("TOKEN_ACCOUNT_DEFINED shield", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.shield({
        publicAmountSol: amount,
        token: "SOL",
        senderTokenAccount: SolanaKeypair.generate().publicKey,
      }),
      UserErrorCode.TOKEN_ACCOUNT_DEFINED,
    );
  });

  it("TOKEN_NOT_FOUND shield", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.shield({ publicAmountSol: amount, token: "SPL" }),
      UserErrorCode.TOKEN_NOT_FOUND,
    );
  });

  it("TOKEN_NOT_FOUND unshield", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.unshield({ amountSol: amount, token: "SPL" }),
      UserErrorCode.TOKEN_NOT_FOUND,
    );
  });

  it("TOKEN_NOT_FOUND transfer", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.unshield({ amountSol: amount, token: "SPL" }),
      UserErrorCode.TOKEN_NOT_FOUND,
    );
  });

  it("NO_PUBLIC_AMOUNTS_PROVIDED unshield", async () => {
    await chai.assert.isRejected(
      user.unshield({ token }),
      CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED,
    );
  });

  it("TOKEN_NOT_FOUND unshield", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.unshield({}),
      UserErrorCode.TOKEN_NOT_FOUND,
    );
  });

  it("SOL_RECIPIENT_UNDEFINED unshield", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.unshield({ token: "SOL", publicAmountSol: new BN(1) }),
      TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
    );

    await chai.assert.isRejected(
      // @ts-ignore
      user.unshield({
        token,
        publicAmountSol: new BN(1),
        publicAmountSpl: new BN(1),
        recipientSpl: SolanaKeypair.generate().publicKey,
      }),
      TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
    );
  });

  it("SPL_RECIPIENT_UNDEFINED unshield", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.unshield({ token, publicAmountSpl: new BN(1) }),
      TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
    );
  });

  it("TOKEN_NOT_FOUND shield", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.shield({ publicAmountSol: SolanaKeypair.generate().publicKey }),
      UserErrorCode.TOKEN_NOT_FOUND,
    );
  });

  it("TOKEN_NOT_FOUND transfer", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.transfer({
        recipient: new Account({ poseidon: POSEIDON }).getPublicKey(),
        amountSol: new BN(1),
        token: "SPL",
      }),
      UserErrorCode.TOKEN_NOT_FOUND,
    );
  });

  it("SHIELDED_RECIPIENT_UNDEFINED transfer", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.transfer({}),
      UserErrorCode.SHIELDED_RECIPIENT_UNDEFINED,
    );
  });

  it("NO_AMOUNTS_PROVIDED transfer", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.transfer({
        recipient: new Account({ poseidon: POSEIDON }).getPublicKey(),
      }),
      UserErrorCode.NO_AMOUNTS_PROVIDED,
    );
  });
});

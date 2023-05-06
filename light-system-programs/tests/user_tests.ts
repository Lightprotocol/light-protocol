import * as anchor from "@coral-xyz/anchor";
import {
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
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
let circomlibjs = require("circomlibjs");

// TODO: add and use  namespaces in SDK
import {
  setUpMerkleTree,
  initLookUpTableFromFile,
  merkleTreeProgramId,
  ADMIN_AUTH_KEYPAIR,
  MINT,
  Provider,
  newAccountWithTokens,
  createTestAccounts,
  confirmConfig,
  User,
  strToArr,
  TOKEN_REGISTRY,
  Account,
  CreateUtxoErrorCode,
  UserErrorCode,
  TransactionErrorCode,
  ADMIN_AUTH_KEY,
  TestRelayer,
  Action,
  TestStateValidator,
} from "light-sdk";

import { BN } from "@coral-xyz/anchor";

var LOOK_UP_TABLE;
var POSEIDON;
var RELAYER: TestRelayer, provider: Provider;

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

  before("init test setup Merkle tree lookup table etc ", async () => {
    let initLog = console.log;
    // console.log = () => {};
    await createTestAccounts(anchorProvider.connection);
    LOOK_UP_TABLE = await initLookUpTableFromFile(anchorProvider);
    await setUpMerkleTree(anchorProvider);
    // console.log = initLog;
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
  });

  it("(user class) shield SPL", async () => {
    let testInputs = {
      amountSpl: 20,
      amountSol: 0,
      token: "USDC",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
    };

    const provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    });

    let res = await provider.provider.connection.requestAirdrop(
      userKeypair.publicKey,
      2_000_000_000,
    );

    await provider.provider.connection.confirmTransaction(res, "confirmed");

    const user: User = await User.init({ provider });

    const testStateValidator = new TestStateValidator({
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

    // TODO: add random amount and amount checks
    await user.provider.latestMerkleTree();

    await testStateValidator.checkTokenShielded();
  });

  it("(user class) shield SOL", async () => {
    let testInputs = {
      amountSpl: 0,
      amountSol: 15,
      token: "SOL",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
    };

    const provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    }); // userKeypair

    let res = await provider.provider.connection.requestAirdrop(
      userKeypair.publicKey,
      4_000_000_000,
    );

    await provider.provider.connection.confirmTransaction(res, "confirmed");

    const user: User = await User.init({ provider });

    const testStateValidator = new TestStateValidator({
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

    // TODO: add random amount and amount checks
    await user.provider.latestMerkleTree();

    await testStateValidator.checkSolShielded();
  });
  // TODO: add test for recipient to nacl recipient
  // this only works by itself because it does not merge a utxo
  // get balance cannot reflect a balance of several utxos apparently
  // not going to fix getBalance since it is already refactored in
  // a subsequent pr
  it.skip("(user class) shield SOL to recipient", async () => {
    const senderAccountSeed = bs58.encode(new Uint8Array(32).fill(7));
    const senderAccount = new Account({
      poseidon: POSEIDON,
      seed: senderAccountSeed,
    });

    let testInputs = {
      amountSpl: 0,
      amountSol: 15,
      token: "SOL",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
      recipientAccount: userKeypair,
      mergedUtxo: false,
    };

    const provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    }); // userKeypair

    let res = await provider.provider.connection.requestAirdrop(
      userKeypair.publicKey,
      4_000_000_000,
    );

    await provider.provider.connection.confirmTransaction(res, "confirmed");

    const userSender: User = await User.init({
      provider,
      seed: senderAccountSeed,
    });
    const userRecipient: User = await User.init({ provider });

    const testStateValidator = new TestStateValidator({
      userSender,
      userRecipient,
      provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();

    await userSender.shield({
      publicAmountSol: testInputs.amountSol,
      token: testInputs.token,
      recipient: userRecipient.account.getPublicKey(),
    });

    // TODO: add random amount and amount checks
    await userRecipient.provider.latestMerkleTree();
    await testStateValidator.checkSolShielded();
  });

  it("(user class) unshield SPL", async () => {
    const solRecipient = SolanaKeypair.generate();

    const testInputs = {
      amountSpl: 1,
      amountSol: 0,
      token: "USDC",
      type: Action.UNSHIELD,
      recipientSpl: solRecipient.publicKey,
      expectedUtxoHistoryLength: 1,
    };

    const provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    }); // userKeypair

    let res = await provider.provider.connection.requestAirdrop(
      userKeypair.publicKey,
      2_000_000_000,
    );

    await provider.provider.connection.confirmTransaction(res, "confirmed");

    // TODO: add test case for if recipient doesnt have account yet -> relayer must create it
    await newAccountWithTokens({
      connection: provider.provider.connection,
      MINT,
      ADMIN_AUTH_KEYPAIR: userKeypair,
      userAccount: solRecipient,
      amount: new anchor.BN(0),
    });

    const user: User = await User.init({ provider });

    const testStateValidator = new TestStateValidator({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();

    await user.unshield({
      publicAmountSpl: testInputs.amountSpl,
      token: testInputs.token,
      recipientSpl: testInputs.recipientSpl,
    });

    await user.provider.latestMerkleTree();

    await testStateValidator.checkTokenUnshielded();
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

    const provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    });

    const recipientAccount = new Account({
      poseidon: POSEIDON,
      seed: testInputs.recipientSeed,
    });

    const user: User = await User.init({ provider });
    const userRecipient: User = await User.init({
      provider,
      seed: testInputs.recipientSeed,
    });

    const testStateValidator = new TestStateValidator({
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
    await testStateValidator.checkTokenTransferred();
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
      LOOK_UP_TABLE = await initLookUpTableFromFile(providerAnchor);
    }

    POSEIDON = await circomlibjs.buildPoseidonOpt();
    amount = 20;
    token = "USDC";

    provider = await await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    }); // userKeypair
    let res = await provider.provider.connection.requestAirdrop(
      userKeypair.publicKey,
      2_000_000_000,
    );
    await provider.provider.connection.confirmTransaction(res, "confirmed");
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

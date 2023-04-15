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
  fetchNullifierAccountInfo,
  Relayer,
} from "light-sdk";

import { BN } from "@coral-xyz/anchor";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

var LOOK_UP_TABLE;
var POSEIDON;
var RELAYER: TestRelayer, provider: Provider;

class BalanceChecker {
  private preShieldedBalances;
  private preTokenBalance;
  private preRecipientTokenBalance;
  private preSolBalance;
  public provider: Provider;

  async fetchAndSaveBalances({
    user,
    provider,
    userSplAccount,
    recipientSplAccount,
  }: {
    user: User;
    provider: Provider;
    userSplAccount?: PublicKey;
    recipientSplAccount?: PublicKey;
  }) {
    this.preShieldedBalances = await user.getBalance({ latest: true });
    if (userSplAccount) {
      this.preTokenBalance =
        await provider.provider.connection.getTokenAccountBalance(
          userSplAccount,
        );
    }
    if (recipientSplAccount) {
      this.preRecipientTokenBalance =
        await provider.provider.connection.getTokenAccountBalance(
          recipientSplAccount,
        );
    }
    this.preSolBalance = await provider.provider.connection.getBalance(
      provider.wallet.publicKey,
    );
    this.provider = provider;
  }

  async assertShieldedTokenBalance(user: User, tokenCtx, amount) {
    const postShieldedBalances = await user.getBalance({ latest: true });

    let tokenBalanceAfter = postShieldedBalances.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );
    let tokenBalancePre = this.preShieldedBalances.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );

    assert.equal(
      tokenBalanceAfter.amount,
      tokenBalancePre.amount.toNumber() +
        amount * tokenCtx?.decimals.toNumber(),
      `shielded balance after ${tokenBalanceAfter.amount} != shield amount ${
        amount * tokenCtx?.decimals.toNumber()
      }`,
    );
  }

  async assertTokenBalance(userSplAccount, amount) {
    const postTokenBalance =
      await this.provider.provider.connection.getTokenAccountBalance(
        userSplAccount,
      );

    assert.equal(
      postTokenBalance.value.uiAmount,
      this.preTokenBalance.value.uiAmount + amount,
      `user token balance after ${postTokenBalance.value.uiAmount} != user token balance before ${this.preTokenBalance.value.uiAmount} - shield amount ${amount}`,
    );
  }

  async assertSolBalance(amount, tokenCtx, tempAccountCost) {
    const postSolBalance = await this.provider.provider.connection.getBalance(
      this.provider.wallet.publicKey,
    );

    assert.equal(
      postSolBalance,
      this.preSolBalance -
        amount * tokenCtx.decimals.toNumber() +
        tempAccountCost,
      `user token balance after ${postSolBalance} != user token balance before ${this.preSolBalance} - shield amount ${amount} sol + tempAccountCost! ${tempAccountCost}`,
    );
  }

  async assertRecipientTokenBalance(recipientSplAccount, amount) {
    const postRecipientTokenBalance =
      await this.provider.provider.connection.getTokenAccountBalance(
        recipientSplAccount,
      );

    assert.equal(
      postRecipientTokenBalance.value.uiAmount,
      this.preRecipientTokenBalance.value.uiAmount + amount,
      `user token balance after ${postRecipientTokenBalance.value.uiAmount} != user token balance before ${this.preRecipientTokenBalance.value.uiAmount} - shield amount ${amount}`,
    );
  }

  async assertShieledSolBalance(user, amount) {
    const postShieldedBalances = await user.getBalance({ latest: true });

    let solBalanceAfter = postShieldedBalances.find(
      (b) => b.tokenAccount.toBase58() === SystemProgram.programId.toString(),
    );
    let solBalancePre = this.preShieldedBalances.find(
      (b) => b.tokenAccount.toBase58() === SystemProgram.programId.toString(),
    );

    assert.equal(
      solBalanceAfter.amount,
      solBalancePre.amount.toNumber() + amount, //+ 2 * 1e9, this MINIMZM
      `shielded sol balance after ${solBalanceAfter.amount} != shield amount 0//2 aka min sol amount (50k)`,
    );
  }
}

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

  const userKeypair = ADMIN_AUTH_KEYPAIR; //new SolanaKeypair();

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
    provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    });
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
    provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    });
    const testInputs = {
      amountSpl: 1,
      amountSol: 0,
      token: "USDC",
      type: Action.UNSHIELD,
      recipientSpl: solRecipient.publicKey,
      expectedUtxoHistoryLength: 1,
    };

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

    provider = await Provider.init({
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

    const balanceChecker = new BalanceChecker();

    await balanceChecker.fetchAndSaveBalances({
      user,
      provider,
    });

    await user.transfer({
      amountSpl: testInputs.amountSpl,
      token: testInputs.token,
      recipient: recipientAccount.getPublicKey(),
    });

    await user.provider.latestMerkleTree();
    await user.getBalance();
    await testStateValidator.checkTokenTransferred();
  });

  it.skip("(user class) transfer SOL", async () => {
    let amount = 1;
    let token = "SOL";
    const shieldedRecipient =
      "19a20668193c0143dd96983ef457404280741339b95695caddd0ad7919f2d434";
    const encryptionPublicKey =
      "LPx24bc92eecaf5e3904bc1f4f731a2b1e0a28adf445e800c4cff112eb7a3f5350b";
    const recipient = new anchor.BN(shieldedRecipient, "hex");
    const recipientEncryptionPublicKey: Uint8Array =
      strToArr(encryptionPublicKey);
    const provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    }); // userKeypair
    // get token from registry
    const tokenCtx = TOKEN_REGISTRY.get(token);

    const user = await User.init({ provider });
    const preShieldedBalance = await user.getBalance({ latest: true });

    await user.transfer({
      amount,
      token,
      recipient,
      recipientEncryptionPublicKey, // TODO: do shielded address
    });

    await user.provider.latestMerkleTree();

    let balance = await user.getBalance({ latest: true });

    // assert that the user's sol shielded balance has decreased by fee
    let solBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === SystemProgram.programId.toString(),
    );
    let solBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === SystemProgram.programId.toString(),
    );

    assert.equal(
      solBalanceAfter.amount,
      solBalancePre.amount - 100000 - amount * tokenCtx.decimals.toNumber(),
      `shielded sol balance after ${solBalanceAfter.amount} != ${solBalancePre.amount} ...unshield amount -fee`,
    );
  });

  it.skip("(user class) unshield SOL", async () => {
    let amount = 1;
    let token = "SOL";
    let recipient = new PublicKey(
      "E7jqevikamCMCda8yCsfNawj57FSotUZuref9MLZpWo1",
    );

    const provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    }); // userKeypair

    const user = await User.init({ provider });
    await user.unshield({ amount, token, recipient });
    // TODO: add random amount and amount checks
  });

  it.skip("(user class) shield SOL to recipient", async () => {
    let amount = 15;
    let token = "SOL";
    const provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    }); // userKeypair
    let res = await provider.provider.connection.requestAirdrop(
      userKeypair.publicKey,
      4_000_000_000,
    );
    const recipientAccount = new Account({
      poseidon: POSEIDON,
      seed: new Uint8Array(32).fill(7).toString(),
    });
    await provider.provider.connection.confirmTransaction(res, "confirmed");
    const userSender: User = await User.init({ provider });
    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    const user: User = await User.init({ provider, seed: seed32 });

    const preShieldedBalance = await user.getBalance({ latest: true });
    const preSolBalance = await provider.provider.connection.getBalance(
      userKeypair.publicKey,
    );
    const previousUtxos = user.utxos;

    await userSender.shield({
      publicAmountSol: amount,
      token,
      recipient: recipientAccount,
    });
    // TODO: add random amount and amount checks
    await user.provider.latestMerkleTree();

    let balance = await user.getBalance({ latest: true });
    let solShieldedBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );
    let solShieldedBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );

    // console.log("solShieldedBalanceAfter", solShieldedBalanceAfter);
    // console.log("solShieldedBalancePre", solShieldedBalancePre);

    // assert that the user's token balance has decreased by the amount shielded
    const postSolBalance = await provider.provider.connection.getBalance(
      userKeypair.publicKey,
    );
    let tempAccountCost = 3502840 - 1255000; //x-y nasty af. underterministic: costs more(y) if shielded SPL before!
    // console.log("postSolBalance", postSolBalance);
    // console.log("preSolBalance", preSolBalance);
    // assert that the user's shielded balance has increased by the amount shielded
    assert.equal(
      solShieldedBalanceAfter.amount.toNumber(),
      solShieldedBalancePre.amount.toNumber() +
        amount * tokenCtx?.decimals.toNumber(),
      `shielded balance after ${
        solShieldedBalanceAfter.amount
      } != shield amount ${amount * tokenCtx?.decimals.toNumber()}`,
    );

    assert.equal(
      postSolBalance,
      preSolBalance - amount * tokenCtx.decimals.toNumber() + tempAccountCost,
      `user token balance after ${postSolBalance} != user token balance before ${preSolBalance} - shield amount ${amount} sol + tempAccountCost! ${tempAccountCost}`,
    );

    assert.notEqual(
      fetchNullifierAccountInfo(user.utxos[0]._nullifier, provider.connection),
      null,
    );

    assert.equal(user.utxos.length, 1);

    const indexedTransactions = await provider.relayer.getIndexedTransactions(
      provider.provider.connection,
    );
    const recentTransaction = indexedTransactions[0];
    assert.equal(
      recentTransaction.amountSol.div(tokenCtx.decimals).toNumber(),
      amount,
    );
    assert.equal(
      recentTransaction.from.toBase58(),
      provider.wallet.publicKey.toBase58(),
    );
    assert.equal(recentTransaction.commitment, user.utxos[0]._commitment);
    assert.equal(recentTransaction.type, Action.SHIELD);
    assert.equal(recentTransaction.relayerFee.toString(), "0");
    assert.equal(
      recentTransaction.relayerRecipientSol.toBase58(),
      PublicKey.default.toBase58(),
    );
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

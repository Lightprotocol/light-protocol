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
  fetchNullifierAccountInfo,
  Action,
  TestStateValidator,
} from "light-sdk";

import { BN } from "@coral-xyz/anchor";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";

var LOOK_UP_TABLE;
var POSEIDON;
var RELAYER;

// TODO: remove deprecated function calls
describe("Test User", () => {
  // Configure the client to use the local cluster.
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

  const provider = anchor.AnchorProvider.local(
    "http://127.0.0.1:8899",
    confirmConfig,
  );
  anchor.setProvider(provider);
  console.log("merkleTreeProgram: ", merkleTreeProgramId.toBase58());

  const userKeypair = ADMIN_AUTH_KEYPAIR; //new SolanaKeypair();
  const testStateValidator = new TestStateValidator();

  before("init test setup Merkle tree lookup table etc ", async () => {
    let initLog = console.log;
    // console.log = () => {};
    await createTestAccounts(provider.connection);
    LOOK_UP_TABLE = await initLookUpTableFromFile(provider);
    await setUpMerkleTree(provider);
    // console.log = initLog;
    POSEIDON = await circomlibjs.buildPoseidonOpt();

    const relayerRecipientSol = SolanaKeypair.generate().publicKey;

    await provider.connection.requestAirdrop(
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
    let amount = 20;
    let token = "USDC";
    console.log("test user wallet: ", userKeypair.publicKey.toBase58());
    const provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    }); // userKeypair
    let res = await provider.provider.connection.requestAirdrop(
      userKeypair.publicKey,
      2_000_000_000,
    );
    // get token
    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);

    const userSplAccount = getAssociatedTokenAddressSync(
      tokenCtx!.tokenAccount,
      ADMIN_AUTH_KEYPAIR.publicKey,
    );

    await provider.provider.connection.confirmTransaction(res, "confirmed");

    const user: User = await User.init(provider);

    await testStateValidator.fetchAndSaveState({
      user,
      provider,
      userSplAccount,
    });
    await user.shield({ publicAmountSpl: amount, token });

    // TODO: add random amount and amount checks
    await user.provider.latestMerkleTree();

    // assert that the user's shielded balance has increased by the amount shielded
    await testStateValidator.assertShieldedTokenBalance(user, tokenCtx, amount);

    // assert that the user's token balance has decreased by the amount shielded
    await testStateValidator.assertTokenBalance(userSplAccount, -amount);

    // assert that the user's sol shielded balance has increased by the additional sol amount
    await testStateValidator.assertShieledSolBalance(user, 150000);

    assert.equal(user.spentUtxos.length, 0);

    assert.notEqual(
      fetchNullifierAccountInfo(user.utxos[0]._nullifier, provider.connection),
      null,
    );

    // assert that recentIndexedTransaction is of type SHIELD and have right values
    await testStateValidator.assertRecentIndexedTransaction({
      amountSpl: amount,
      tokenCtx,
      user,
      type: Action.SHIELD,
    });
  });

  it("(user class) shield SOL", async () => {
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
    await provider.provider.connection.confirmTransaction(res, "confirmed");
    const user: User = await User.init(provider);
    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);

    await testStateValidator.fetchAndSaveState({ user, provider });

    await user.shield({ publicAmountSol: amount, token });
    // TODO: add random amount and amount checks
    await user.provider.latestMerkleTree();

    let tempAccountCost = 3502840 - 1255000; //x-y nasty af. underterministic: costs more(y) if shielded SPL before!

    // assert that the user's shielded balance has increased by the amount shielded
    await testStateValidator.assertShieledSolBalance(
      user,
      amount * tokenCtx?.decimals.toNumber(),
    );

    // assert that the user's token balance has decreased by the amount shielded
    await testStateValidator.assertSolBalance(
      amount,
      tokenCtx,
      tempAccountCost,
    );

    // assert that user utxos are spent and updated correctly
    await testStateValidator.assertUserUtxos(user);

    // assert that recentIndexedTransaction is of type SHIELD and have right values
    await testStateValidator.assertRecentIndexedTransaction({
      amountSol: amount,
      tokenCtx,
      user,
      type: Action.SHIELD,
    });
  });

  it("(user class) unshield SPL", async () => {
    let amount = 1;
    let token = "USDC";
    let solRecipient = SolanaKeypair.generate();
    const provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    }); // userKeypair
    let res = await provider.provider.connection.requestAirdrop(
      userKeypair.publicKey,
      2_000_000_000,
    );

    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    const recipientSplAccount = getAssociatedTokenAddressSync(
      tokenCtx!.tokenAccount,
      solRecipient.publicKey,
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

    const user: User = await User.init(provider);
    await testStateValidator.fetchAndSaveState({
      user,
      provider,
      recipientSplAccount,
    });

    await user.unshield({
      publicAmountSpl: amount,
      token,
      recipientSpl: solRecipient.publicKey,
    });

    await user.provider.latestMerkleTree();

    // assert that the user's shielded token balance has decreased by the amount unshielded
    await testStateValidator.assertShieldedTokenBalance(
      user,
      tokenCtx,
      -amount,
    );

    // assert that the recipient token balance has increased by the amount shielded
    await testStateValidator.assertRecipientTokenBalance(
      recipientSplAccount,
      amount,
    );

    // assert that the user's sol shielded balance has increased by the additional sol amount
    const minimumBalance = 150000;
    const tokenAccountFee = 500_000;
    await testStateValidator.assertShieledSolBalance(
      user,
      -minimumBalance - tokenAccountFee,
    );

    // assert that user utxos are spent and updated correctly
    await testStateValidator.assertUserUtxos(user);

    // assert that recentIndexedTransaction is of type UNSHIELD and have right values
    await testStateValidator.assertRecentIndexedTransaction({
      amountSpl: amount,
      user,
      type: Action.UNSHIELD,
      tokenCtx,
      recipientSplAccount,
    });
  });

  it("(user class) transfer SPL", async () => {
    let amountSpl = 1;
    const token = "USDC";
    const provider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
    }); // userKeypair
    // const shieldedRecipient =
    //   "19a20668193c0143dd96983ef457404280741339b95695caddd0ad7919f2d434";
    // const encryptionPublicKey =
    //   "LPx24bc92eecaf5e3904bc1f4f731a2b1e0a28adf445e800c4cff112eb7a3f5350b";
    const recipientAccount = new Account({
      poseidon: POSEIDON,
      seed: new Uint8Array(32).fill(9).toString(),
    });

    const recipientAccountFromPubkey = Account.fromPubkey(
      recipientAccount.pubkey.toBuffer(),
      recipientAccount.encryptionKeypair.publicKey,
      POSEIDON,
    );
    // get token from registry
    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);

    const user: User = await User.init(provider);

    await testStateValidator.fetchAndSaveState({
      user,
      provider,
    });

    await user.transfer({
      amountSpl,
      token,
      recipient: recipientAccountFromPubkey,
    });

    await user.provider.latestMerkleTree();

    // assert that the user's shielded balance has decreased by the amount transferred
    await testStateValidator.assertShieldedTokenBalance(
      user,
      tokenCtx,
      -amountSpl,
    );

    // assert that the user's sol shielded balance has decreased by fee
    await testStateValidator.assertShieledSolBalance(
      user,
      -provider.relayer.relayerFee.toNumber(),
    );

    // assert that user utxos are spent and updated correctly
    await testStateValidator.assertUserUtxos(user);

    // assert that recentIndexedTransaction is of type SHIELD and have right values
    await testStateValidator.assertRecentIndexedTransaction({
      user,
      tokenCtx,
      amountSol: 0,
      amountSpl: 0,
      type: Action.TRANSFER,
    });
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
    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);

    const user = await User.init(provider);
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

    const user = await User.init(provider);
    await user.unshield({ amount, token, recipient });
    // TODO: add random amount and amount checks
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
  console.log("merkleTreeProgram: ", merkleTreeProgramId.toBase58());

  const userKeypair = ADMIN_AUTH_KEYPAIR; //new SolanaKeypair();

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
    user = await User.init(provider);
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
        recipient: new Account({ poseidon: POSEIDON }),
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
      user.transfer({ recipient: new Account({ poseidon: POSEIDON }) }),
      UserErrorCode.NO_AMOUNTS_PROVIDED,
    );
  });
});

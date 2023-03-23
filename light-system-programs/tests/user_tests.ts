import * as anchor from "@coral-xyz/anchor";
import { Keypair as SolanaKeypair, PublicKey, SystemProgram } from "@solana/web3.js";
import _ from "lodash";
import { assert } from "chai";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);

let circomlibjs = require("circomlibjs");

// TODO: add and use  namespaces in SDK
import {
  Utxo,
  setUpMerkleTree,
  initLookUpTableFromFile,
  merkleTreeProgramId,
  ADMIN_AUTH_KEYPAIR,
  MINT,
  Provider,
  newAccountWithTokens,
  createTestAccounts,
  confirmConfig,
  Relayer,
  User,
  strToArr,
  TOKEN_REGISTRY,
  updateMerkleTreeForTest,
  createOutUtxos,
  Account,
  CreateUtxoErrorCode,
  UserErrorCode,
  TransactionErrorCode,
  ADMIN_AUTH_KEY,
} from "light-sdk";

import { BN } from "@coral-xyz/anchor";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";

var LOOK_UP_TABLE;
var POSEIDON;

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

  before("init test setup Merkle tree lookup table etc ", async () => {
    let initLog = console.log;
    // console.log = () => {};
    await createTestAccounts(provider.connection);
    LOOK_UP_TABLE = await initLookUpTableFromFile(provider);
    await setUpMerkleTree(provider);
    // console.log = initLog;
    POSEIDON = await circomlibjs.buildPoseidonOpt();
  });

  it("(user class) shield SPL", async () => {
    let amount = 20;
    let token = "USDC";
    console.log("test user wallet: ", userKeypair.publicKey.toBase58());
    const provider = await Provider.native(userKeypair); // userKeypair
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
    const preTokenBalance =
      await provider.provider.connection.getTokenAccountBalance(userSplAccount);

    await provider.provider.connection.confirmTransaction(res, "confirmed");
    const user: User = await User.load(provider);
    const preShieldedBalance = await user.getBalance({ latest: true });

    await user.shield({ publicAmountSpl: amount, token });

    try {
      console.log("updating merkle tree...");
      let initLog = console.log;
      console.log = () => {};
      await updateMerkleTreeForTest(provider.provider?.connection!);
      console.log = initLog;
      console.log("✔️updated merkle tree!");
    } catch (e) {
      console.log(e);
      throw new Error("Failed to update merkle tree!");
    }
    // TODO: add random amount and amount checks
    await user.provider.latestMerkleTree();
    let balance;
    try {
      balance = await user.getBalance({ latest: true });
    } catch (e) {
      throw new Error(`ayayay ${e}`);
    }
    let tokenBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );
    let tokenBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );

    // assert that the user's shielded balance has increased by the amount shielded
    assert.equal(
      tokenBalanceAfter.amount,
      tokenBalancePre.amount.toNumber() + amount * tokenCtx?.decimals.toNumber(),
      `shielded balance after ${tokenBalanceAfter.amount} != shield amount ${
        amount * tokenCtx?.decimals.toNumber()
      }`,
    );

    // assert that the user's token balance has decreased by the amount shielded
    const postTokenBalance =
      await provider.provider.connection.getTokenAccountBalance(userSplAccount);
    assert.equal(
      postTokenBalance.value.uiAmount,
      preTokenBalance.value.uiAmount - amount,
      `user token balance after ${postTokenBalance.value.uiAmount} != user token balance before ${preTokenBalance.value.uiAmount} - shield amount ${amount}`,
    );

    // assert that the user's sol shielded balance has increased by the additional sol amount
    let solBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === "11111111111111111111111111111111",
    );
    let solBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === "11111111111111111111111111111111",
    );
    // console.log("solBalancePre", solBalancePre);
    // console.log("solBalanceAfter", solBalanceAfter);
    assert.equal(
      solBalanceAfter.amount,
      solBalancePre.amount.toNumber() + 150000, //+ 2 * 1e9, this MINIMZM
      `shielded sol balance after ${solBalanceAfter.amount} != shield amount 0//2 aka min sol amount (50k)`,
    );
  });

  it("(user class) shield SOL", async () => {
    let amount = 15;
    let token = "SOL";
    const provider = await Provider.native(userKeypair);
    let res = await provider.provider.connection.requestAirdrop(
      userKeypair.publicKey,
      4_000_000_000,
    );
    await provider.provider.connection.confirmTransaction(res, "confirmed");
    const user: User = await User.load(provider);
    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    const preShieldedBalance = await user.getBalance({ latest: true });
    const preSolBalance = await provider.provider.connection.getBalance(
      userKeypair.publicKey,
    );

    await user.shield({ publicAmountSol: amount, token });
    // TODO: add random amount and amount checks
    try {
      console.log("updating merkle tree...");
      let initLog = console.log;
      console.log = () => {};
      await updateMerkleTreeForTest(
        provider.provider?.connection!,
        // provider.provider,
      );
      console.log = initLog;
      console.log("✔️updated merkle tree!");
    } catch (e) {
      console.log(e);
      throw new Error("Failed to update merkle tree!");
    }
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
      solShieldedBalancePre.amount.toNumber() + amount * tokenCtx?.decimals.toNumber(),
      `shielded balance after ${
        solShieldedBalanceAfter.amount
      } != shield amount ${amount * tokenCtx?.decimals.toNumber()}`,
    );

    assert.equal(
      postSolBalance,
      preSolBalance - amount * tokenCtx.decimals.toNumber() + tempAccountCost,
      `user token balance after ${postSolBalance} != user token balance before ${preSolBalance} - shield amount ${amount} sol + tempAccountCost! ${tempAccountCost}`,
    );
  });

  it("(user class) unshield SPL", async () => {
    let amount = 1;
    let token = "USDC";
    let solRecipient = SolanaKeypair.generate();
    const provider = await Provider.native(userKeypair); // userKeypair
    let res = await provider.provider.connection.requestAirdrop(
      userKeypair.publicKey,
      2_000_000_000,
    );

    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    const recipientSplBalance = getAssociatedTokenAddressSync(
      tokenCtx!.tokenAccount,
      solRecipient.publicKey,
    );

    await provider.provider.connection.confirmTransaction(res, "confirmed");
    let preTokenBalance = { value: { uiAmount: 0 } };
    // TODO: add test case for if recipient doesnt have account yet -> relayer must create it
    await newAccountWithTokens({
      connection: provider.provider.connection,
      MINT,
      ADMIN_AUTH_KEYPAIR: userKeypair,
      userAccount: solRecipient,
      amount: new anchor.BN(0),
    });
    try {
      preTokenBalance =
        await provider.provider.connection.getTokenAccountBalance(
          recipientSplBalance,
        );
    } catch (e) {
      console.log(
        "recipient account does not exist yet (creating as part of user class)",
      );
    }

    const user: User = await User.load(provider);
    const preShieldedBalance = await user.getBalance({ latest: true });

    await user.unshield({ publicAmountSpl: amount, token, recipientSpl: solRecipient.publicKey });

    try {
      console.log("updating merkle tree...");
      let initLog = console.log;
      console.log = () => {};
      await updateMerkleTreeForTest(provider.provider?.connection!);
      console.log = initLog;
      console.log("✔️updated merkle tree!");
    } catch (e) {
      console.log(e);
      throw new Error("Failed to update merkle tree!");
    }
    await user.provider.latestMerkleTree();

    let balance = await user.getBalance({ latest: true });
    let tokenBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );
    let tokenBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );

    // assert that the user's shielded balance has decreased by the amount unshielded
    assert.equal(
      tokenBalanceAfter.amount.toNumber(),
      tokenBalancePre.amount.toNumber() - amount * tokenCtx?.decimals.toNumber(), // TODO: check that fees go ?
      `shielded balance after ${tokenBalanceAfter.amount} != unshield amount ${
        amount * tokenCtx?.decimals.toNumber()
      }`,
    );

    // assert that the user's token balance has decreased by the amount shielded
    const postTokenBalance =
      await provider.provider.connection.getTokenAccountBalance(
        recipientSplBalance,
      );
    assert.equal(
      postTokenBalance.value.uiAmount,
      preTokenBalance.value.uiAmount + amount,
      `user token balance after ${postTokenBalance.value.uiAmount} != user token balance before ${preTokenBalance.value.uiAmount} + unshield amount ${amount}`,
    );

    // assert that the user's sol shielded balance has increased by the additional sol amount
    let solBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === "11111111111111111111111111111111",
    );
    let solBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === "11111111111111111111111111111111",
    );
    const minimumBalance = 150000;
    const tokenAccountFee = 500_000
    assert.equal(
      solBalanceAfter.amount.toNumber(),
      solBalancePre.amount.toNumber() - minimumBalance - tokenAccountFee, // FIXME: no fees being charged here apparently
      `shielded sol balance after ${solBalanceAfter.amount} != unshield amount ${solBalancePre.amount.toNumber() - minimumBalance - tokenAccountFee}`,
    );
    // TODO: add checks for relayer fee recipient (log all balance changes too...)
  });

  it("(user class) transfer SPL", async () => {
    let amountSpl = 1;
    const token = "USDC";
    const provider = await Provider.native(userKeypair); // userKeypair
    const shieldedRecipient =
      "19a20668193c0143dd96983ef457404280741339b95695caddd0ad7919f2d434";
    const encryptionPublicKey =
      "LPx24bc92eecaf5e3904bc1f4f731a2b1e0a28adf445e800c4cff112eb7a3f5350b";


    const recipientAccount = Account.fromPubkey(
      strToArr(shieldedRecipient),
      strToArr(encryptionPublicKey),
      POSEIDON,
    );
    // get token from registry
    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    const recipient = new anchor.BN(shieldedRecipient, "hex");
    const recipientEncryptionPublicKey: Uint8Array =
      strToArr(encryptionPublicKey);

    const user: User = await User.load(provider);
    const preShieldedBalance = await user.getBalance({ latest: true });

    await user.transfer({
      amountSpl,
      token,
      recipient: recipientAccount,
    });

    try {
      console.log("updating merkle tree...");
      let initLog = console.log;
      console.log = () => {};
      await updateMerkleTreeForTest(provider.provider?.connection!);
      console.log = initLog;
      console.log("✔️updated merkle tree!");
    } catch (e) {
      console.log(e);
      throw new Error("Failed to update merkle tree!");
    }
    await user.provider.latestMerkleTree();

    let balance = await user.getBalance({ latest: true });
    let tokenBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );
    let tokenBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );
    // assert that the user's shielded balance has decreased by the amount transferred
    assert.equal(
      tokenBalanceAfter.amount.toNumber(),
      tokenBalancePre.amount.toNumber() - amountSpl * tokenCtx?.decimals.toNumber(), // TODO: check that fees go ?
      `shielded balance after ${tokenBalanceAfter.amount} != unshield amount ${
        amountSpl * tokenCtx?.decimals.toNumber()
      }`,
    );
    // assert that the user's sol shielded balance has decreased by fee
    let solBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === SystemProgram.programId.toBase58(),
    );
    let solBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === SystemProgram.programId.toBase58(),
    );
    const minimumChangeUtxoAmounts = 50000 * 3;
    assert.equal(
      solBalanceAfter.amount.toNumber(),
      solBalancePre.amount.toNumber() - 100000, // FIXME: no fees being charged here apparently
      `shielded sol balance after ${solBalanceAfter.amount} != unshield amount -fee -minimumSplUtxoChanges`,
    );
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
    const provider = await Provider.native(userKeypair);
    // get token from registry
    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);

    const user = await User.load(provider);
    const preShieldedBalance = await user.getBalance({ latest: true });

    await user.transfer({
      amount,
      token,
      recipient,
      recipientEncryptionPublicKey, // TODO: do shielded address
    });
    try {
      console.log("updating merkle tree...");
      let initLog = console.log;
      console.log = () => {};
      await updateMerkleTreeForTest(
        provider.provider?.connection!,
        // provider.provider,
      );
      console.log = initLog;
      console.log("✔️updated merkle tree!");
    } catch (e) {
      console.log(e);
      throw new Error("Failed to update merkle tree!");
    }
    await user.provider.latestMerkleTree();

    let balance = await user.getBalance({ latest: true });

    // assert that the user's sol shielded balance has decreased by fee
    let solBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === "11111111111111111111111111111111",
    );
    let solBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === "11111111111111111111111111111111",
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
    const provider = await Provider.native(userKeypair);
    const user = await User.load(provider);
    await user.unshield({ amount, token, recipient });
    try {
      console.log("updating merkle tree...");
      let initLog = console.log;
      console.log = () => {};
      await updateMerkleTreeForTest(
        provider.provider?.connection!,
        // provider.provider,
      );
      console.log = initLog;
      console.log("✔️updated merkle tree!");
    } catch (e) {
      console.log(e);
      throw new Error("Failed to update merkle tree!");
    }
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
    if((await providerAnchor.connection.getBalance(ADMIN_AUTH_KEY)) === 0) {
      await createTestAccounts(providerAnchor.connection);
      LOOK_UP_TABLE = await initLookUpTableFromFile(providerAnchor);
    }

    POSEIDON = await circomlibjs.buildPoseidonOpt();
    amount = 20;
    token = "USDC";

    provider = await Provider.native(userKeypair); // userKeypair
    let res = await provider.provider.connection.requestAirdrop(
      userKeypair.publicKey,
      2_000_000_000,
    );
    await provider.provider.connection.confirmTransaction(res, "confirmed");
    user = await User.load(provider);
  });

  it("NO_PUBLIC_AMOUNTS_PROVIDED shield", async () => {
    await chai.assert.isRejected(
      user.shield({ token }),
      CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED
    )
  });

  it("TOKEN_UNDEFINED shield", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.shield({ publicAmountSpl: amount }),
      UserErrorCode.TOKEN_UNDEFINED
    )
  });

  it("INVALID_TOKEN shield", async () => {
    
    await chai.assert.isRejected(
      // @ts-ignore
      user.shield({ publicAmountSpl: amount , token: "SOL"}),
      UserErrorCode.INVALID_TOKEN
    )
  });

  it("TOKEN_ACCOUNT_DEFINED shield", async () => {
    
    await chai.assert.isRejected(
      // @ts-ignore
      user.shield({ publicAmountSol: amount , token: "SOL", senderTokenAccount: SolanaKeypair.generate().publicKey}),
      UserErrorCode.TOKEN_ACCOUNT_DEFINED
    )
  });

  it("TOKEN_NOT_FOUND shield", async () => {
    
    await chai.assert.isRejected(
      // @ts-ignore
      user.shield({ publicAmountSol: amount , token: "SPL"}),
      UserErrorCode.TOKEN_NOT_FOUND
    )
  });

  it("TOKEN_NOT_FOUND unshield", async () => {
    
    await chai.assert.isRejected(
      // @ts-ignore
      user.unshield({ amountSol: amount , token: "SPL"}),
      UserErrorCode.TOKEN_NOT_FOUND
    )
  });

  it("TOKEN_NOT_FOUND transfer", async () => {
    
    await chai.assert.isRejected(
      // @ts-ignore
      user.unshield({ amountSol: amount , token: "SPL"}),
      UserErrorCode.TOKEN_NOT_FOUND
    )
  });

  it("NO_PUBLIC_AMOUNTS_PROVIDED unshield", async () => {
    
    await chai.assert.isRejected(
      user.unshield({ token }),
      CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED
    )
  });

  it("TOKEN_NOT_FOUND unshield", async () => {
    
    await chai.assert.isRejected(
      // @ts-ignore
      user.unshield({ }),
      UserErrorCode.TOKEN_NOT_FOUND
    )
  });

  it("SOL_RECIPIENT_UNDEFINED unshield", async () => {
    
    await chai.assert.isRejected(
      // @ts-ignore
      user.unshield({ token: "SOL", publicAmountSol: new BN(1)}),
      TransactionErrorCode.SOL_RECIPIENT_UNDEFINED
    )

    await chai.assert.isRejected(
      // @ts-ignore
      user.unshield({ token, publicAmountSol: new BN(1), publicAmountSpl: new BN(1), recipientSpl: SolanaKeypair.generate().publicKey}),
      TransactionErrorCode.SOL_RECIPIENT_UNDEFINED
    )
  });

  it("SPL_RECIPIENT_UNDEFINED unshield", async () => {
    
    await chai.assert.isRejected(
      // @ts-ignore
      user.unshield({ token, publicAmountSpl: new BN(1)}),
      TransactionErrorCode.SPL_RECIPIENT_UNDEFINED
    )
  });

  it("TOKEN_NOT_FOUND shield", async () => {
    
    await chai.assert.isRejected(
      // @ts-ignore
      user.shield({ publicAmountSol: SolanaKeypair.generate().publicKey}),
      UserErrorCode.TOKEN_NOT_FOUND
    )
  });

  it("TOKEN_NOT_FOUND transfer", async () => {
    
    await chai.assert.isRejected(
      // @ts-ignore
      user.transfer({recipient: new Account({poseidon: POSEIDON}), amountSol: new BN(1) }),
      UserErrorCode.TOKEN_NOT_FOUND
    )
  });

  it("SHIELDED_RECIPIENT_UNDEFINED transfer", async () => {
  
    await chai.assert.isRejected(
      // @ts-ignore
      user.transfer({ }),
      UserErrorCode.SHIELDED_RECIPIENT_UNDEFINED
    )
  });

  it("NO_AMOUNTS_PROVIDED transfer", async () => {

    await chai.assert.isRejected(
      // @ts-ignore
      user.transfer({recipient: new Account({poseidon: POSEIDON}) }),
      UserErrorCode.NO_AMOUNTS_PROVIDED
    )
  });



})

import * as anchor from "@coral-xyz/anchor";
import { Keypair as SolanaKeypair, PublicKey } from "@solana/web3.js";
import _ from "lodash";
import { assert } from "chai";

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
} from "light-sdk";

import { BN } from "@coral-xyz/anchor";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";

var LOOK_UP_TABLE;
var POSEIDON;

// TODO: remove deprecated function calls
describe("verifier_program", () => {
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

  it("(createOutUtxos) unshield in:1 SPL ", async () => {
    let amount = 3;
    let token = "USDC";
    let tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    if (!tokenCtx) throw new Error("Token not supported!");
    amount = amount * tokenCtx.decimals;
    const provider = await Provider.native(userKeypair);
    const user = await User.load(provider);
    let utxo1 = new Utxo({
      poseidon: POSEIDON,
      assets: [
        new PublicKey("11111111111111111111111111111111"),
        tokenCtx.tokenAccount,
      ],
      amounts: [new BN(1e8), new BN(5 * tokenCtx.decimals)],
    });

    let outUtxos = createOutUtxos({
      mint: tokenCtx.tokenAccount,
      amount: -amount,
      inUtxos: [utxo1],
      extraSolAmount: 0,
      poseidon: POSEIDON,
      account: user.account,
    });

    assert.equal(
      outUtxos[0].amounts[0].toNumber(),
      utxo1.amounts[0].toNumber(),
      `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`,
    );
    assert.equal(
      outUtxos[0].amounts[1].toNumber(),
      utxo1.amounts[1].toNumber() - amount,
      `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${
        utxo1.amounts[1].toNumber() - amount * tokenCtx.decimals
      }`,
    );
  });
  it("(createOutUtxos) unshield in:1SOL + 1SPL should merge 2-1", async () => {
    let amount = 3;
    let token = "USDC";
    let tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    if (!tokenCtx) throw new Error("Token not supported!");
    amount = amount * tokenCtx.decimals;
    const provider = await Provider.native(userKeypair);
    const user = await User.load(provider);
    let utxo1 = new Utxo({
      poseidon: POSEIDON,
      assets: [
        new PublicKey("11111111111111111111111111111111"),
        tokenCtx.tokenAccount,
      ],
      amounts: [new BN(1e8), new BN(5 * tokenCtx.decimals)],
    });
    let utxoSol = new Utxo({
      poseidon: POSEIDON,
      assets: [new PublicKey("11111111111111111111111111111111")],
      amounts: [new BN(1e6)],
    });
    let outUtxos = createOutUtxos({
      mint: tokenCtx.tokenAccount,
      amount: -amount,
      inUtxos: [utxo1, utxoSol],
      extraSolAmount: 0,
      poseidon: POSEIDON,
      account: user.account,
    });
    assert.equal(
      outUtxos[0].amounts[0].toNumber(),
      utxo1.amounts[0].toNumber() + utxoSol.amounts[0].toNumber(),
      `${outUtxos[0].amounts[0]} fee != ${
        utxo1.amounts[0].toNumber() + utxoSol.amounts[0].toNumber()
      }`,
    );
    assert.equal(
      outUtxos[0].amounts[1].toNumber(),
      utxo1.amounts[1].toNumber() - amount,
      `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${
        utxo1.amounts[1].toNumber() - amount * tokenCtx.decimals
      }`,
    );
  });
  it("(createOutUtxos) unshield in:1SPL + 1SPL should merge 2-1", async () => {
    let amount = 3;
    let token = "USDC";
    let tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    if (!tokenCtx) throw new Error("Token not supported!");
    amount = amount * tokenCtx.decimals;
    const provider = await Provider.native(userKeypair);
    const user = await User.load(provider);
    let utxo1 = new Utxo({
      poseidon: POSEIDON,
      assets: [
        new PublicKey("11111111111111111111111111111111"),
        tokenCtx.tokenAccount,
      ],
      amounts: [new BN(1e8), new BN(5 * tokenCtx.decimals)],
    });
    let utxo2 = new Utxo({
      poseidon: POSEIDON,
      assets: [
        new PublicKey("11111111111111111111111111111111"),
        tokenCtx.tokenAccount,
      ],
      amounts: [new BN(1e8), new BN(5 * tokenCtx.decimals)],
    });

    let outUtxos = createOutUtxos({
      mint: tokenCtx.tokenAccount,
      amount: -amount,
      inUtxos: [utxo1, utxo2],
      extraSolAmount: 0,
      poseidon: POSEIDON,
      account: user.account,
    });
    assert.equal(
      outUtxos[0].amounts[0].toNumber(),
      utxo1.amounts[0].toNumber() + utxo2.amounts[0].toNumber(),
      `${outUtxos[0].amounts[0]} fee != ${
        utxo1.amounts[0].toNumber() + utxo2.amounts[0].toNumber()
      }`,
    );
    assert.equal(
      outUtxos[0].amounts[1].toNumber(),
      utxo1.amounts[1].toNumber() + utxo2.amounts[1].toNumber() - amount,
      `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${
        utxo1.amounts[1].toNumber() - amount
      }`,
    );
  });
  it("(createOutUtxos) transfer in:1 SPL ", async () => {
    let amount = 3;
    let token = "USDC";
    const shieldedRecipient =
      "19a20668193c0143dd96983ef457404280741339b95695caddd0ad7919f2d434";
    const encryptionPublicKey =
      "LPx24bc92eecaf5e3904bc1f4f731a2b1e0a28adf445e800c4cff112eb7a3f5350b";

    const recipient = new anchor.BN(shieldedRecipient, "hex");
    const recipientEncryptionPublicKey: Uint8Array =
      strToArr(encryptionPublicKey);
    let tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    if (!tokenCtx) throw new Error("Token not supported!");
    amount = amount * tokenCtx.decimals;
    const provider = await Provider.native(userKeypair);
    const user = await User.load(provider);
    let utxo1 = new Utxo({
      poseidon: POSEIDON,
      assets: [
        new PublicKey("11111111111111111111111111111111"),
        tokenCtx.tokenAccount,
      ],
      amounts: [new BN(1e8), new BN(5 * tokenCtx.decimals)],
    });
    const relayer = new Relayer(
      // ADMIN_AUTH_KEYPAIR.publicKey,
      provider.nodeWallet!.publicKey,
      provider.lookUpTable!,
      SolanaKeypair.generate().publicKey,
      new anchor.BN(100000),
    );
    let outUtxos = createOutUtxos({
      mint: tokenCtx.tokenAccount,
      amount: amount,
      inUtxos: [utxo1],
      recipient: recipient,
      recipientEncryptionPublicKey: recipientEncryptionPublicKey,
      relayer: relayer,
      extraSolAmount: 0,
      poseidon: POSEIDON,
      account: user.account,
    });
    assert.equal(
      outUtxos[1].amounts[0].toNumber(),
      utxo1.amounts[0].toNumber() -
        relayer.relayerFee.toNumber() -
        outUtxos[0].amounts[0].toNumber(),
      `${outUtxos[1].amounts[0]} fee != ${
        utxo1.amounts[0].toNumber() -
        relayer.relayerFee.toNumber() -
        outUtxos[0].amounts[0].toNumber()
      }`,
    );

    assert.equal(
      outUtxos[1].amounts[1].toNumber(),
      utxo1.amounts[1].toNumber() - amount,
      `${outUtxos[1].amounts[1].toNumber()}  spl !=  ${
        utxo1.amounts[1].toNumber() - amount
      }`,
    );
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
    const user = await User.load(provider);
    const preShieldedBalance = await user.getBalance({ latest: true });

    await user.shield({ amount, token, extraSolAmount: 2 });

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
    let balance = await user.getBalance({ latest: true });
    let tokenBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );
    let tokenBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );

    // assert that the user's shielded balance has increased by the amount shielded
    assert.equal(
      tokenBalanceAfter.amount,
      tokenBalancePre.amount + amount * tokenCtx?.decimals,
      `shielded balance after ${tokenBalanceAfter.amount} != shield amount ${
        amount * tokenCtx?.decimals
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
    assert.equal(
      solBalanceAfter.amount,
      solBalancePre.amount + 2 * 1e9,
      `shielded sol balance after ${solBalanceAfter.amount} != shield amount 2`,
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
    const user = await User.load(provider);
    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    const preShieldedBalance = await user.getBalance({ latest: true });
    const preSolBalance = await provider.provider.connection.getBalance(
      userKeypair.publicKey,
    );

    await user.shield({ amount, token });
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
    let balance = await user.getBalance({ latest: true });
    let solShieldedBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );
    let solShieldedBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );

    // assert that the user's shielded balance has increased by the amount shielded
    assert.equal(
      solShieldedBalanceAfter.amount,
      solShieldedBalancePre.amount + amount * tokenCtx?.decimals,
      `shielded balance after ${
        solShieldedBalanceAfter.amount
      } != shield amount ${amount * tokenCtx?.decimals}`,
    );

    // assert that the user's token balance has decreased by the amount shielded
    const postSolBalance = await provider.provider.connection.getBalance(
      userKeypair.publicKey,
    );
    let tempAccountCost = 3502840 - 1255000; //x-y nasty af. underterministic: costs more(y) if shielded SPL before!

    assert.equal(
      postSolBalance,
      preSolBalance - amount * tokenCtx.decimals + tempAccountCost,
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

    const user = await User.load(provider);
    const preShieldedBalance = await user.getBalance({ latest: true });

    await user.unshield({ amount, token, recipient: solRecipient.publicKey });

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

    let balance = await user.getBalance({ latest: true });
    let tokenBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );
    let tokenBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );

    // assert that the user's shielded balance has decreased by the amount unshielded
    assert.equal(
      tokenBalanceAfter.amount,
      tokenBalancePre.amount - amount * tokenCtx?.decimals, // TODO: check that fees go ?
      `shielded balance after ${tokenBalanceAfter.amount} != unshield amount ${
        amount * tokenCtx?.decimals
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
    assert.equal(
      solBalanceAfter.amount,
      solBalancePre.amount, // FIXME: no fees being charged here apparently
      `shielded sol balance after ${solBalanceAfter.amount} != unshield amount -100000`,
    );
    // TODO: add checks for relayer fee recipient (log all balance changes too...)
  });

  it("(user class) transfer SPL", async () => {
    let amount = 1;
    const token = "USDC";
    const provider = await Provider.native(userKeypair); // userKeypair
    const shieldedRecipient =
      "19a20668193c0143dd96983ef457404280741339b95695caddd0ad7919f2d434";
    const encryptionPublicKey =
      "LPx24bc92eecaf5e3904bc1f4f731a2b1e0a28adf445e800c4cff112eb7a3f5350b";

    // get token from registry
    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    const recipient = new anchor.BN(shieldedRecipient, "hex");
    const recipientEncryptionPublicKey: Uint8Array =
      strToArr(encryptionPublicKey);

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
      await updateMerkleTreeForTest(provider.provider?.connection!);
      console.log = initLog;
      console.log("✔️updated merkle tree!");
    } catch (e) {
      console.log(e);
      throw new Error("Failed to update merkle tree!");
    }
    let balance = await user.getBalance({ latest: true });
    let tokenBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );
    let tokenBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );
    // assert that the user's shielded balance has decreased by the amount transferred
    assert.equal(
      tokenBalanceAfter.amount,
      tokenBalancePre.amount - amount * tokenCtx?.decimals, // TODO: check that fees go ?
      `shielded balance after ${tokenBalanceAfter.amount} != unshield amount ${
        amount * tokenCtx?.decimals
      }`,
    );
    // assert that the user's sol shielded balance has decreased by fee
    let solBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === "11111111111111111111111111111111",
    );
    let solBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === "11111111111111111111111111111111",
    );
    const minimumChangeUtxoAmounts = 50000 * 2;
    assert.equal(
      solBalanceAfter.amount,
      solBalancePre.amount - 100000 - minimumChangeUtxoAmounts, // FIXME: no fees being charged here apparently
      `shielded sol balance after ${solBalanceAfter.amount} != unshield amount -fee -minimumSplUtxoChanges`,
    );
  });

  it("(user class) transfer SOL", async () => {
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
      solBalancePre.amount - 100000 - amount * tokenCtx.decimals,
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

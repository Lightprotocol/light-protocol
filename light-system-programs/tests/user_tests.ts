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
  KEYPAIR_PRIVKEY,
  newAccountWithTokens,
  createTestAccounts,
  confirmConfig,
  Relayer,
  verifierStorageProgramId,
  User,
  IDL_VERIFIER_PROGRAM_STORAGE,
  strToArr,
  TOKEN_REGISTRY,
} from "light-sdk";

import { BN } from "@coral-xyz/anchor";
import { Account } from "light-sdk/lib/account";

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
    let outUtxos = user.createOutUtxos({
      mint: tokenCtx.tokenAccount,
      amount: -amount,
      inUtxos: [utxo1],
      extraSolAmount: 0,
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
    let outUtxos = user.createOutUtxos({
      mint: tokenCtx.tokenAccount,
      amount: -amount,
      inUtxos: [utxo1, utxoSol],
      extraSolAmount: 0,
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
    let outUtxos = user.createOutUtxos({
      mint: tokenCtx.tokenAccount,
      amount: -amount,
      inUtxos: [utxo1, utxo2],
      extraSolAmount: 0,
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
  it.skip("(createOutUtxos) transfer in:1 SPL ", async () => {
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
    let outUtxos = user.createOutUtxos({
      mint: tokenCtx.tokenAccount,
      amount: amount,
      inUtxos: [utxo1],
      recipient: recipient,
      recipientEncryptionPublicKey: recipientEncryptionPublicKey,
      relayer: relayer,
      extraSolAmount: 0,
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
  it.only("(user class) shield SPL", async () => {
    let amount = 20;
    let token = "USDC";
    console.log("test user wallet: ", userKeypair.publicKey.toBase58());
    const provider = await Provider.native(userKeypair); // userKeypair
    let res = await provider.provider.connection.requestAirdrop(
      userKeypair.publicKey,
      2_000_000_000,
    );

    await provider.provider.connection.confirmTransaction(res, "confirmed");
    const user = await User.load(provider);
    await user.shield({ amount, token });
    // TODO: add random amount and amount checks
    // let balance = await user.getBalance({ latest: true });
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
    await user.shield({ amount, token });
    // TODO: add random amount and amount checks
  });
  it("(user class) unshield SPL", async () => {
    let amount = 1;
    let token = "USDC";
    let solRecipient = SolanaKeypair.generate();
    console.log("test user wallet: ", userKeypair.publicKey.toBase58());
    const provider = await Provider.native(userKeypair); // userKeypair
    let recipientTokenAccount = await newAccountWithTokens({
      connection: provider.provider.connection,
      MINT,
      ADMIN_AUTH_KEYPAIR: userKeypair,
      userAccount: solRecipient,
      amount: new anchor.BN(0),
    });
    // console.log("recipientTokenAccount: ", recipientTokenAccount.toBase58());
    let res = await provider.provider.connection.requestAirdrop(
      userKeypair.publicKey,
      2_000_000_000,
    );

    await provider.provider.connection.confirmTransaction(res, "confirmed");
    const user = await User.load(provider);
    await user.unshield({ amount, token, recipient: solRecipient.publicKey });

    // TODO: add random amount and amount checks
    // let recipientBalanceAfter =
    //   await provider.provider.connection.getTokenAccountBalance(
    //     recipientTokenAccount,
    //   );
    // console.log("recipientBalanceAfter: ", recipientBalanceAfter);
    // let balance = await user.getBalance({ latest: true });
  });
  it("(user class) transfer SPL", async () => {
    let amount = 1;
    let token = "USDC";
    console.log("test user wallet: ", userKeypair.publicKey.toBase58());
    const provider = await Provider.native(userKeypair); // userKeypair
    const shieldedRecipient =
      "19a20668193c0143dd96983ef457404280741339b95695caddd0ad7919f2d434";
    const encryptionPublicKey =
      "LPx24bc92eecaf5e3904bc1f4f731a2b1e0a28adf445e800c4cff112eb7a3f5350b";

    const recipient = new anchor.BN(shieldedRecipient, "hex");
    const recipientEncryptionPublicKey: Uint8Array =
      strToArr(encryptionPublicKey);

    const user = await User.load(provider);
    await user.transfer({
      amount,
      token,
      recipient,
      recipientEncryptionPublicKey, // TODO: do shielded address
    });
    // TODO: add balance checks
    // let balance = await user.getBalance({ latest: true });
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
    const user = await User.load(provider);
    await user.transfer({
      amount,
      token,
      recipient,
      recipientEncryptionPublicKey, // TODO: do shielded address
    });

    // TODO: add random amount, recipient and amount checks
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
    // TODO: add random amount and amount checks
  });
});

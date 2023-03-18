import { assert, expect } from "chai";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
import { BN } from "@coral-xyz/anchor";
// Load chai-as-promised support
chai.use(chaiAsPromised);
let circomlibjs = require("circomlibjs");
import {
  SystemProgram,
  Keypair as SolanaKeypair,
  PublicKey,
} from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { it } from "mocha";
import { buildPoseidonOpt, buildBabyjub, buildEddsa } from "circomlibjs";
import { Scalar } from "ffjavascript";

import { Account } from "../src/account";
import {
  FEE_ASSET,
  functionalCircuitTest,
  hashAndTruncateToCircuit,
  Provider as LightProvider,
  MINT,
  Transaction,
  UtxoError,
  UtxoErrorCode,
  TransactionError,
  TransactionErrorCode,
  ProviderErrorCode,
  Provider,
  TransactionParameters,
  VerifierZero,
  Action,
  Relayer,
  AccountError,
  AccountErrorCode,
  TransactionParametersErrorCode,
  createOutUtxos,
  strToArr,
  ADMIN_AUTH_KEYPAIR,
  TOKEN_REGISTRY,
  User,
  Utxo,
} from "../src";
const { blake2b } = require("@noble/hashes/blake2b");
const b2params = { dkLen: 32 };
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
let seed32 = new Uint8Array(32).fill(1).toString();

describe("Test selectInUtxos Functional", () => {
  var poseidon, eddsa, babyJub, F, k0: Account, k00: Account, kBurner: Account;
  const userKeypair = ADMIN_AUTH_KEYPAIR; //new SolanaKeypair();
  const mockPublicKey = SolanaKeypair.generate().publicKey;

  before(async () => {
    poseidon = await circomlibjs.buildPoseidonOpt();
    eddsa = await buildEddsa();
    babyJub = await buildBabyjub();
    F = babyJub.F;
    k0 = new Account({ poseidon, seed: seed32 });
    k00 = new Account({ poseidon, seed: seed32 });
    kBurner = Account.createBurner(poseidon, seed32, new anchor.BN("0"));
  });

  it("(createOutUtxos) unshield in:1 SPL ", async () => {
    let amount = 3;
    let token = "USDC";
    let tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    if (!tokenCtx) throw new Error("Token not supported!");
    amount = amount * tokenCtx.decimals;
    let utxo1 = new Utxo({
      poseidon,
      assets: [SystemProgram.programId, tokenCtx.tokenAccount],
      amounts: [new BN(1e8), new BN(5 * tokenCtx.decimals)],
    });

    let outUtxos = createOutUtxos({
      mint: tokenCtx.tokenAccount,
      splAmount: -amount,
      inUtxos: [utxo1],
      solAmount: 0,
      poseidon,
      senderAccount: k0,
      action: Action.SHIELD,
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
    let utxo1 = new Utxo({
      poseidon,
      assets: [SystemProgram.programId, tokenCtx.tokenAccount],
      amounts: [new BN(1e8), new anchor.BN(5 * tokenCtx.decimals)],
    });
    let utxoSol = new Utxo({
      poseidon,
      assets: [SystemProgram.programId],
      amounts: [new BN(1e6)],
    });
    let outUtxos = createOutUtxos({
      mint: tokenCtx.tokenAccount,
      splAmount: -amount,
      inUtxos: [utxo1, utxoSol],
      solAmount: 0,
      poseidon,
      senderAccount: k0,
      action: Action.SHIELD,
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
    let utxo1 = new Utxo({
      poseidon,
      assets: [SystemProgram.programId, tokenCtx.tokenAccount],
      amounts: [new BN(1e8), new BN(5 * tokenCtx.decimals)],
    });
    let utxo2 = new Utxo({
      poseidon,
      assets: [SystemProgram.programId, tokenCtx.tokenAccount],
      amounts: [new BN(1e8), new BN(5 * tokenCtx.decimals)],
    });

    let outUtxos = createOutUtxos({
      mint: tokenCtx.tokenAccount,
      splAmount: -amount,
      inUtxos: [utxo1, utxo2],
      solAmount: 0,
      poseidon,
      senderAccount: k0,
      action: Action.SHIELD,
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
    let utxo1 = new Utxo({
      poseidon,
      assets: [SystemProgram.programId, tokenCtx.tokenAccount],
      amounts: [new BN(1e8), new BN(5 * tokenCtx.decimals)],
    });
    const relayer = new Relayer(
      // ADMIN_AUTH_KEYPAIR.publicKey,
      mockPublicKey,
      mockPublicKey,
      SolanaKeypair.generate().publicKey,
      new anchor.BN(100000),
    );
    let outUtxos = createOutUtxos({
      mint: tokenCtx.tokenAccount,
      splAmount: amount,
      inUtxos: [utxo1],
      recipient: recipient,
      recipientEncryptionPublicKey: recipientEncryptionPublicKey,
      relayer: relayer,
      solAmount: 0,
      poseidon,
      senderAccount: k0,
      action: Action.SHIELD,
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
});

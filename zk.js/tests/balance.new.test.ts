import { assert } from "chai";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { it } from "mocha";
import { expect } from "chai";
const circomlibjs = require("circomlibjs");
const { buildPoseidonOpt } = circomlibjs;
const chai = require("chai");

const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);

import {
  FEE_ASSET,
  Provider as LightProvider,
  MINT,
  Utxo,
  Account,
  TOKEN_REGISTRY,
  BN_0,
  getTokenDataByMint,
  initTokenBalance,
  isSPLUtxo,
  addUtxoToBalance,
  Balance_new,
  updateTokenBalanceWithUtxo,
  serializeBalance,
  deserializeBalance,
  spendUtxo,
  TokenBalance_new,
} from "../src";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

describe("Balance", () => {
  const seed32 = bs58.encode(new Uint8Array(32).fill(1));
  const shieldAmount = 20_000;
  const shieldFeeAmount = 10_000;

  let poseidon: any,
    lightProvider: LightProvider,
    shieldUtxo1: Utxo,
    solTestUtxo1: Utxo,
    solTestUtxo2: Utxo,
    keypair: Account;
  beforeEach(async () => {
    poseidon = await buildPoseidonOpt();
    keypair = new Account({ poseidon: poseidon, seed: seed32 });
    lightProvider = await LightProvider.loadMock();
    shieldUtxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(shieldFeeAmount), new BN(shieldAmount)],
      publicKey: keypair.pubkey,
      index: 1,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    solTestUtxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, SystemProgram.programId],
      amounts: [new BN(shieldAmount), new BN(0)],
      publicKey: keypair.pubkey,
      index: 3,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    solTestUtxo2 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, SystemProgram.programId],
      amounts: [new BN(shieldAmount * 1.5), new BN(0)],
      publicKey: keypair.pubkey,
      index: 5,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
  });

  describe("getTokenDataByMint", () => {
    it("should return the correct token data", () => {
      const solTokenData = getTokenDataByMint(
        SystemProgram.programId,
        TOKEN_REGISTRY,
      );
      assert.equal(solTokenData.symbol, "SOL");
    });

    it("should throw an error if the token is not found", () => {
      const NO_TOKEN = new PublicKey(
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYC",
      );
      expect(() => getTokenDataByMint(NO_TOKEN, TOKEN_REGISTRY)).to.throw();
    });
    it("should throw an error if the registry is empty", () => {
      expect(() =>
        getTokenDataByMint(SystemProgram.programId, new Map()),
      ).to.throw();
    });
  });

  describe("initTokenBalance", () => {
    it("should initialize a token balance correctly", () => {
      const tokenData = getTokenDataByMint(
        SystemProgram.programId,
        TOKEN_REGISTRY,
      );
      const utxos: Utxo[] = [solTestUtxo1];

      const tokenBalance = initTokenBalance(tokenData, utxos);

      assert(tokenBalance.amount.eq(BN_0)); // is sol utxo
      assert(tokenBalance.lamports.eq(utxos[0].amounts[0]));
      assert.deepEqual(tokenBalance.data, tokenData);
      assert.deepEqual(tokenBalance.utxos, utxos);
    });

    it("should throw an error if the mint does not match tokendata", () => {
      const tokenData = getTokenDataByMint(
        SystemProgram.programId,
        TOKEN_REGISTRY,
      );
      const utxos: Utxo[] = [shieldUtxo1]; // diff mints

      expect(() => {
        initTokenBalance(tokenData, utxos);
      }).to.throw();
    });
    it("should throw an error if two diff mint UTXOs", () => {
      const tokenData = getTokenDataByMint(
        SystemProgram.programId,
        TOKEN_REGISTRY,
      );
      const utxos: Utxo[] = [shieldUtxo1, solTestUtxo1]; // diff mints

      expect(() => {
        initTokenBalance(tokenData, utxos);
      }).to.throw();
    });

    it("should initialize a token balance correctly without utxos", () => {
      const tokenData = getTokenDataByMint(
        SystemProgram.programId,
        TOKEN_REGISTRY,
      );
      const tokenBalance = initTokenBalance(tokenData);

      assert.equal(tokenBalance.amount, BN_0);
      assert.equal(tokenBalance.lamports, BN_0);
      assert.deepEqual(tokenBalance.data, tokenData);
      assert.deepEqual(tokenBalance.utxos, []);
    });
  });

  describe("isSPLUtxo", () => {
    it("should return true if the utxo is an SPL utxo", () => {
      const result = isSPLUtxo(shieldUtxo1);
      assert.equal(result, true);
    });

    it("should return false if the utxo is not an SPL utxo", () => {
      const result = isSPLUtxo(solTestUtxo1);
      assert.equal(result, false);
    });
  });

  describe("addUtxoToBalance", () => {
    let balance: Balance_new;

    before(() => {
      // set up initial balance with 1 SOL tokenbalance
      const tokenData = getTokenDataByMint(
        SystemProgram.programId,
        TOKEN_REGISTRY,
      );
      const tokenBalance = initTokenBalance(tokenData, [solTestUtxo1]);
      balance = {
        tokenBalances: new Map([
          [tokenBalance.data.mint.toBase58(), tokenBalance],
        ]),
      };
    });

    it("should add a new mint UTXO to balance correctly", () => {
      const utxo: Utxo = shieldUtxo1;

      // fresh tokenbalance,
      const result = addUtxoToBalance(utxo, balance, poseidon);

      assert.equal(result, true);
      assert.deepEqual(
        balance.tokenBalances.get(utxo.assets[1].toString())!.utxos[0],
        utxo,
      );
      assert.deepEqual(
        balance.tokenBalances.get(utxo.assets[1].toString())!.amount,
        utxo.amounts[1],
      );
    });

    it("should update the SOL token balance with another UTXO correctly", () => {
      const utxo: Utxo = solTestUtxo2; // solTestUtxo2 = sol, but different
      const result = addUtxoToBalance(utxo, balance, poseidon);

      assert.equal(result, true);
      assert.deepEqual(
        balance.tokenBalances.get(utxo.assets[0].toString())!.utxos[1], // 2nd
        utxo,
      );
      assert.deepEqual(
        balance.tokenBalances.get(utxo.assets[0].toString())!.lamports,
        utxo.amounts[0].add(solTestUtxo1.amounts[0]), // 2 sol utxos
      );
    });
  });

  describe("updateTokenBalanceWithUtxo", () => {
    let tokenBalance: TokenBalance_new;

    before(() => {
      const tokenData = getTokenDataByMint(
        SystemProgram.programId,
        TOKEN_REGISTRY,
      );
      tokenBalance = initTokenBalance(tokenData, [solTestUtxo1]);
    });

    it("should update the token balance with a new UTXO correctly", () => {
      const utxo: Utxo = solTestUtxo2;

      const result = updateTokenBalanceWithUtxo(utxo, tokenBalance, poseidon);

      assert.equal(result, true);
      assert.deepEqual(tokenBalance.utxos[1], utxo);
      assert.deepEqual(
        tokenBalance.lamports,
        utxo.amounts[0].add(solTestUtxo1.amounts[0]),
      );
      assert.deepEqual(tokenBalance.amount, utxo.amounts[1] ?? BN_0);
    });

    it("should not update the token balance with the same UTXO", () => {
      const utxo: Utxo = solTestUtxo2;

      updateTokenBalanceWithUtxo(utxo, tokenBalance, poseidon);

      const result = updateTokenBalanceWithUtxo(utxo, tokenBalance, poseidon);

      assert.equal(result, false);
    });
  });

  describe("serializeBalance and deserializeBalance", () => {
    it("should serialize and deserialize a balance correctly", async () => {
      // Initialize a balance
      const tokenData = getTokenDataByMint(
        SystemProgram.programId,
        TOKEN_REGISTRY,
      );
      const tokenBalance = initTokenBalance(tokenData, [solTestUtxo1]);
      const balance: Balance_new = {
        tokenBalances: new Map([
          [tokenBalance.data.mint.toBase58(), tokenBalance],
        ]),
      };

      // Serialize
      const serializedBalance = await serializeBalance(balance);

      // Deserialize
      const deserializedBalance: Balance_new = deserializeBalance(
        serializedBalance,
        TOKEN_REGISTRY,
        lightProvider,
      );

      // Check that the deserialized balance matches the original balance
      assert(
        customDeepEqual(
          deserializedBalance.tokenBalances.get(
            tokenBalance.data.mint.toBase58(),
          ),
          tokenBalance,
        ),
      );
    });
  });

  describe("spendUtxo", () => {
    it("should spend a UTXO correctly and return true", () => {
      const tokenData = getTokenDataByMint(
        SystemProgram.programId,
        TOKEN_REGISTRY,
      );
      const utxo = solTestUtxo1;
      const tokenBalance = initTokenBalance(tokenData, [utxo]);
      const balance: Balance_new = {
        tokenBalances: new Map([
          [tokenBalance.data.mint.toBase58(), tokenBalance],
        ]),
      };

      const commitment = utxo.getCommitment(poseidon);
      const result = spendUtxo([balance], commitment);
      assert.equal(result, true);

      const updatedTokenBalance = balance.tokenBalances.get(
        tokenBalance.data.mint.toBase58(),
      )!;
      assert.equal(updatedTokenBalance.utxos.length, 0);
      assert.equal(updatedTokenBalance.lamports.toString(), "0");
      assert.equal(updatedTokenBalance.amount.toString(), "0");
    });

    it("should return false when trying to spend a UTXO that has already been spent", () => {
      const tokenData = getTokenDataByMint(
        SystemProgram.programId,
        TOKEN_REGISTRY,
      );
      const tokenBalance = initTokenBalance(tokenData, [solTestUtxo1]);
      const balance: Balance_new = {
        tokenBalances: new Map([
          [tokenBalance.data.mint.toBase58(), tokenBalance],
        ]),
      };

      const commitment = shieldUtxo1.getCommitment(poseidon);
      spendUtxo([balance], commitment);
      const result = spendUtxo([balance], commitment);
      assert.equal(result, false);
    });
  });
});

/// compare BNs and PublicKeys by value
function customDeepEqual(a, b) {
  if (
    (BN.isBN(a) && BN.isBN(b) && a.eq(b)) ||
    (a instanceof PublicKey && b instanceof PublicKey && a.equals(b))
  ) {
    return true;
  }

  if (Array.isArray(a) && Array.isArray(b)) {
    return (
      a.length === b.length &&
      a.every((val, index) => customDeepEqual(val, b[index]))
    );
  }

  if (
    typeof a === "object" &&
    typeof b === "object" &&
    a !== null &&
    b !== null
  ) {
    return (
      Object.keys(a).length === Object.keys(b).length &&
      Object.keys(a).every((key) => customDeepEqual(a[key], b[key]))
    );
  }

  return a === b;
}

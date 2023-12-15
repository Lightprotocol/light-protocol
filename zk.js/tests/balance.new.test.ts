import { assert } from "chai";
import { Connection, PublicKey, SystemProgram } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { it } from "mocha";
import { expect } from "chai";
import { Hasher } from "@lightprotocol/account.rs";
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
  updateTokenBalanceWithUtxo,
  serializeBalance,
  deserializeBalance,
  spendUtxo,
  MerkleTreeConfig,
  NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
  UTXO_PREFIX_LENGTH,
  fetchNullifierAccountInfo,
} from "../src";

import { Balance, TokenBalance } from "../src/types/balance";

import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { WasmHasher } from "@lightprotocol/account.rs";
import {
  findSpentUtxos,
  initBalance,
  sortUtxos,
  tryDecryptNewUtxos,
} from "../src/balance/balance";

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
    keypair: Account,
    hasher: Hasher;
  beforeEach(async () => {
    hasher = await WasmHasher.getInstance();
    poseidon = await buildPoseidonOpt();
    keypair = new Account({ hasher, seed: seed32 });
    lightProvider = await LightProvider.loadMock();
    shieldUtxo1 = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(shieldFeeAmount), new BN(shieldAmount)],
      publicKey: keypair.pubkey,
      index: 1,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    solTestUtxo1 = new Utxo({
      hasher,
      assets: [FEE_ASSET, SystemProgram.programId],
      amounts: [new BN(shieldAmount - 10), new BN(0)],
      publicKey: keypair.pubkey,
      index: 3,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    // smaller:
    solTestUtxo2 = new Utxo({
      hasher,
      assets: [FEE_ASSET, SystemProgram.programId],
      amounts: [new BN(shieldAmount * 0.8), new BN(0)], // maintaining order
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

      assert(tokenBalance.splAmount.eq(BN_0)); // is sol utxo
      assert(tokenBalance.lamports.eq(utxos[0].amounts[0]));
      assert.deepEqual(tokenBalance.tokenData, tokenData);
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

      assert.equal(tokenBalance.splAmount, BN_0);
      assert.equal(tokenBalance.lamports, BN_0);
      assert.deepEqual(tokenBalance.tokenData, tokenData);
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
    let balance: Balance;

    before(() => {
      // set up initial balance with 1 SOL tokenbalance
      const tokenData = getTokenDataByMint(
        SystemProgram.programId,
        TOKEN_REGISTRY,
      );
      const tokenBalance = initTokenBalance(tokenData, [solTestUtxo1]);
      balance = {
        tokenBalances: new Map([
          [tokenBalance.tokenData.mint.toBase58(), tokenBalance],
        ]),
        lastSyncedSlot: 0,
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
        balance.tokenBalances.get(utxo.assets[1].toString())!.splAmount,
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
    let tokenBalance: TokenBalance;

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
      assert.deepEqual(tokenBalance.splAmount, utxo.amounts[1] ?? BN_0);
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
      const balance: Balance = {
        tokenBalances: new Map([
          [tokenBalance.tokenData.mint.toBase58(), tokenBalance],
        ]),
        lastSyncedSlot: 0,
      };

      // Serialize
      const serializedBalance = await serializeBalance(balance);

      // Deserialize
      const deserializedBalance: Balance = deserializeBalance(
        serializedBalance,
        TOKEN_REGISTRY,
        lightProvider,
        hasher,
      );

      // Check that the deserialized balance matches the original balance
      assert(
        customDeepEqual(
          deserializedBalance.tokenBalances.get(
            tokenBalance.tokenData.mint.toBase58(),
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
      const balance: Balance = {
        tokenBalances: new Map([
          [tokenBalance.tokenData.mint.toBase58(), tokenBalance],
        ]),
        lastSyncedSlot: 0,
      };

      const commitment = utxo.getCommitment(poseidon);
      const result = spendUtxo(balance, commitment);
      assert.equal(result, true);

      const updatedTokenBalance = balance.tokenBalances.get(
        tokenBalance.tokenData.mint.toBase58(),
      )!;
      assert.equal(updatedTokenBalance.utxos.length, 0);
      assert.equal(updatedTokenBalance.lamports.toString(), "0");
      assert.equal(updatedTokenBalance.splAmount.toString(), "0");
    });

    it("should return false when trying to spend a UTXO that has already been spent", () => {
      const tokenData = getTokenDataByMint(
        SystemProgram.programId,
        TOKEN_REGISTRY,
      );
      const tokenBalance = initTokenBalance(tokenData, [solTestUtxo1]);
      const balance: Balance = {
        tokenBalances: new Map([
          [tokenBalance.tokenData.mint.toBase58(), tokenBalance],
        ]),
        lastSyncedSlot: 0,
      };

      const commitment = shieldUtxo1.getCommitment(poseidon);
      spendUtxo(balance, commitment);
      const result = spendUtxo(balance, commitment);
      assert.equal(result, false);
    });
  });

  describe("sortUtxos", () => {
    it("should sort big to small", () => {
      const utxos = [solTestUtxo2, solTestUtxo1];
      const sortedUtxos = [solTestUtxo1, solTestUtxo2];
      sortUtxos(utxos);
      assert.deepEqual(utxos, sortedUtxos);
    });

    it("should sort utxos correctly with a single utxo", () => {
      const utxos = [solTestUtxo1];
      const sortedUtxos = [solTestUtxo1];
      sortUtxos(utxos);
      assert.deepEqual(utxos, sortedUtxos);
    });
    it("should sort big to small (nochange)", () => {
      const utxos = [solTestUtxo1, solTestUtxo2];
      const sortedUtxos = [solTestUtxo1, solTestUtxo2];
      sortUtxos(utxos);
      assert.deepEqual(utxos, sortedUtxos);
    });

    it("should break if different mints", () => {
      const utxos = [solTestUtxo1, shieldUtxo1];
      expect(() => sortUtxos(utxos)).to.throw();
    });
  });

  describe.only("syncBalance", () => {
    it.skip("should findSpentUtxos return empty balance", async () => {
      const balance = initBalance();
      console.log("LIGHTPROVIDER", lightProvider);
      // const connection = new Connection("http://127.0.0.1:8899");

      // Add UTXOs to balance
      addUtxoToBalance(solTestUtxo1, balance, hasher);
      addUtxoToBalance(solTestUtxo2, balance, hasher);

      // Now you can use the mocked connection object in your tests
      // const result = await findSpentUtxos(balance, connection, keypair, hasher);

      // Add assertions to check if the function correctly identified the spent UTXO
      // assert.notInclude(
      //   result.tokenBalances.get(solTestUtxo1.assets[0].toString())!.utxos,
      //   solTestUtxo1,
      // ); // utxo1 should be removed from balance
      // assert.include(
      //   result.tokenBalances.get(solTestUtxo2.assets[0].toString())!.utxos,
      //   solTestUtxo2,
      // ); // utxo2 should still be in balance
    });

    it.skip("should findSpentUtxos return balance with 1 utxo", async () => {
      const balance = initBalance();
      const connection = lightProvider.provider.connection;
      const _balance = await findSpentUtxos(
        balance,
        connection,
        keypair,
        hasher,
      );
      assert.equal(_balance.tokenBalances.size, 0);
    });

    it.skip("Test Decrypt Balance 2 and 4 utxos", async () => {
      const provider = await LightProvider.loadMock();
      const assetLookupTable = provider.lookUpTables.assetLookupTable;
      const account = new Account({ hasher, seed: seed32 });
      for (let j = 2; j < 4; j += 2) {
        const utxos: Utxo[] = [];
        let encryptedUtxos: any[] = [];
        for (let index = 0; index < j; index++) {
          const shieldAmount = index;
          const shieldFeeAmount = index;
          const utxo = new Utxo({
            hasher,
            assets: [FEE_ASSET, MINT],
            amounts: [new BN(shieldFeeAmount), new BN(shieldAmount)],
            publicKey: account.pubkey,
            index: index,
            assetLookupTable: provider.lookUpTables.assetLookupTable,
            blinding: new BN(1),
          });
          utxos.push(utxo);
          const encryptedUtxo = await utxo.encrypt({
            hasher,
            account,
            merkleTreePdaPublicKey:
              MerkleTreeConfig.getTransactionMerkleTreePda(),
            compressed: true,
          });
          encryptedUtxos = [...encryptedUtxos, ...encryptedUtxo];
        }
        const indexedTransactions = [
          {
            leaves: utxos.map((utxo) =>
              new BN(utxo.getCommitment(hasher)).toBuffer("be", 32),
            ),
            firstLeafIndex: "0",
            encryptedUtxos,
          },
        ];
        const decryptedUtxos: Array<Utxo> = new Array<Utxo>();
        for (const trx of indexedTransactions) {
          const leftLeafIndex = new BN(trx.firstLeafIndex).toNumber();
          for (let index = 0; index < trx.leaves.length; index += 2) {
            const leafLeft = trx.leaves[index];
            const leafRight = trx.leaves[index + 1];
            let encBytes = Buffer.from(
              trx.encryptedUtxos.slice(
                (index / 2) * 240,
                (index / 2) * 240 +
                  NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH +
                  UTXO_PREFIX_LENGTH,
              ),
            );
            let decryptedUtxo = await Utxo.decrypt({
              hasher,
              encBytes,
              account,
              index: leftLeafIndex + index,
              commitment: leafLeft,
              aes: true,
              merkleTreePdaPublicKey:
                MerkleTreeConfig.getTransactionMerkleTreePda(),
              assetLookupTable,
              merkleProof:
                provider.solMerkleTree!.merkleTree.path(leftLeafIndex)
                  .pathElements,
            });
            assert(decryptedUtxo.error === null, "Can't decrypt utxo");
            if (decryptedUtxo.value !== null) {
              decryptedUtxos.push(decryptedUtxo.value);
            }

            encBytes = Buffer.from(
              trx.encryptedUtxos.slice(
                (index / 2) * 240 + 120 + UTXO_PREFIX_LENGTH,
                (index / 2) * 240 +
                  NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH +
                  120 +
                  UTXO_PREFIX_LENGTH,
              ),
            );
            decryptedUtxo = await Utxo.decrypt({
              hasher,
              encBytes,
              account,
              index: leftLeafIndex + index + 1,
              commitment: leafRight,
              aes: true,
              merkleTreePdaPublicKey:
                MerkleTreeConfig.getTransactionMerkleTreePda(),
              assetLookupTable,
              merkleProof: provider.solMerkleTree!.merkleTree.path(
                leftLeafIndex + 1,
              ).pathElements,
            });
            assert(decryptedUtxo.error === null, "Can't decrypt utxo");
            if (decryptedUtxo.value !== null) {
              decryptedUtxos.push(decryptedUtxo.value);
            }
          }
        }
        utxos.map((utxo, index) => {
          Utxo.equal(hasher, utxo, decryptedUtxos[index]!);
        });
      }
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

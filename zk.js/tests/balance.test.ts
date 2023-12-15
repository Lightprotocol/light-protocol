import { assert } from "chai";
import { SystemProgram } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { it } from "mocha";

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
  TokenUtxoBalance,
  Balance,
  TOKEN_REGISTRY,
  NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
  MerkleTreeConfig,
  BN_0,
  UTXO_PREFIX_LENGTH,
} from "../src";
import { WasmHasher, Hasher } from "@lightprotocol/account.rs";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

describe("Utxo Functional", () => {
  const seed32 = bs58.encode(new Uint8Array(32).fill(1));
  const shieldAmount = 20_000;
  const shieldFeeAmount = 10_000;

  let hasher: Hasher,
    lightProvider: LightProvider,
    shieldUtxo1: Utxo,
    keypair: Account;
  before(async () => {
    hasher = await WasmHasher.getInstance();
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
  });

  it("Test Balance moveToSpentUtxos", async () => {
    const balance: Balance = {
      tokenBalances: new Map([
        [SystemProgram.programId.toBase58(), TokenUtxoBalance.initSol()],
      ]),
      totalSolBalance: BN_0,
      programBalances: new Map(),
      nftBalances: new Map(),
    };
    const tokenBalanceUsdc = new TokenUtxoBalance(TOKEN_REGISTRY.get("USDC")!);
    balance.tokenBalances.set(
      tokenBalanceUsdc.tokenData.mint.toBase58(),
      tokenBalanceUsdc,
    );

    balance.tokenBalances
      .get(MINT.toBase58())
      ?.addUtxo(shieldUtxo1.getCommitment(hasher), shieldUtxo1, "utxos");

    const utxo = balance.tokenBalances
      .get(MINT.toBase58())
      ?.utxos.get(shieldUtxo1.getCommitment(hasher));
    Utxo.equal(hasher, utxo!, shieldUtxo1);
    assert.equal(
      balance.tokenBalances.get(MINT.toBase58())?.totalBalanceSol.toString(),
      shieldUtxo1.amounts[0].toString(),
    );
    assert.equal(
      balance.tokenBalances.get(MINT.toBase58())?.totalBalanceSpl.toString(),
      shieldUtxo1.amounts[1].toString(),
    );
    assert.equal(
      balance.tokenBalances.get(SystemProgram.programId.toBase58())?.spentUtxos
        .size,
      0,
    );

    balance.tokenBalances
      .get(MINT.toBase58())
      ?.moveToSpentUtxos(shieldUtxo1.getCommitment(hasher));
    assert.equal(
      balance.tokenBalances.get(MINT.toBase58())?.totalBalanceSol.toString(),
      "0",
    );
    assert.equal(
      balance.tokenBalances.get(MINT.toBase58())?.totalBalanceSpl.toString(),
      "0",
    );
    assert.equal(
      balance.tokenBalances.get(MINT.toBase58())?.spentUtxos.size,
      1,
    );

    assert.equal(balance.tokenBalances.get(MINT.toBase58())?.utxos.size, 0);

    const _shieldUtxo1 = balance.tokenBalances
      .get(MINT.toBase58())
      ?.spentUtxos.get(shieldUtxo1.getCommitment(hasher));
    Utxo.equal(hasher, _shieldUtxo1!, shieldUtxo1);
  });

  // this test is mock of the syncState function
  it("Test Decrypt Balance 2 and 4 utxos", async () => {
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
          let decryptedUtxo = await Utxo.decryptUnchecked({
            hasher,
            encBytes,
            account,
            index: leftLeafIndex + index,
            commitment: leafLeft,
            aes: true,
            merkleTreePdaPublicKey:
              MerkleTreeConfig.getTransactionMerkleTreePda(),
            assetLookupTable,
            merkleProof: [],
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
          decryptedUtxo = await Utxo.decryptUnchecked({
            hasher,
            encBytes,
            account,
            index: leftLeafIndex + index + 1,
            commitment: leafRight,
            aes: true,
            merkleTreePdaPublicKey:
              MerkleTreeConfig.getTransactionMerkleTreePda(),
            assetLookupTable,
            merkleProof: [],
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

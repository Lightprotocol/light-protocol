import { assert } from "chai";
import { OutUtxo } from "../../src/index";
import { Utxo } from "../../src";

export function compareOutUtxos(utxo1: OutUtxo, utxo2: OutUtxo): void {
  assert.strictEqual(
    utxo1.publicKey.toString(),
    utxo2.publicKey.toString(),
    "publicKey does not match",
  );

  if (utxo1.encryptionPublicKey || utxo2.encryptionPublicKey) {
    assert.deepStrictEqual(
      utxo1.encryptionPublicKey?.toString(),
      utxo2.encryptionPublicKey?.toString(),
      "encryptionPublicKey does not match",
    );
  }

  assert.strictEqual(
    utxo1.amounts.length,
    utxo2.amounts.length,
    "amounts array length does not match",
  );
  utxo1.amounts.forEach((amount, index) =>
    assert(amount.eq(utxo2.amounts[index]), `amounts[${index}] does not match`),
  );

  assert.strictEqual(
    utxo1.assets.length,
    utxo2.assets.length,
    "assets array length does not match",
  );
  utxo1.assets.forEach((asset, index) =>
    assert.strictEqual(
      asset.toString(),
      utxo2.assets[index].toString(),
      `assets[${index}] does not match`,
    ),
  );

  assert.deepEqual(
    utxo1.assetsCircuit,
    utxo2.assetsCircuit,
    "assetsCircuit does not match",
  );
  assert.strictEqual(
    utxo1.blinding.toString(),
    utxo2.blinding.toString(),
    "blinding does not match",
  );
  assert.strictEqual(utxo1.poolType, utxo2.poolType, "poolType does not match");
  assert.strictEqual(
    utxo1.transactionVersion,
    utxo2.transactionVersion,
    "transactionVersion does not match",
  );
  assert.strictEqual(
    utxo1.isFillingUtxo,
    utxo2.isFillingUtxo,
    "isFillingUtxo does not match",
  );
  assert.strictEqual(
    utxo1.utxoDataHash.toString(),
    utxo2.utxoDataHash.toString(),
    "utxoDataHash does not match",
  );
  assert.strictEqual(utxo1.utxoHash, utxo2.utxoHash, "utxoHash does not match");
}

export function compareUtxos(utxo1: Utxo, utxo2: Utxo): void {
  assert.strictEqual(
    utxo1.publicKey.toString(),
    utxo2.publicKey.toString(),
    "publicKey does not match",
  );

  assert.deepStrictEqual(
    utxo1.nullifier.toString(),
    utxo2.nullifier.toString(),
    "nullifier does not match",
  );
  assert.deepStrictEqual(
    utxo1.merkleProof.toString(),
    utxo2.merkleProof.toString(),
    "merkleProof does not match",
  );
  assert.deepStrictEqual(
    utxo1.merkleTreeLeafIndex.toString(),
    utxo2.merkleTreeLeafIndex.toString(),
    "merkleTreeLeafIndex does not match",
  );

  if (utxo1.utxoData || utxo2.utxoData) {
    assert.deepStrictEqual(
      utxo1.utxoData.toString(),
      utxo2.utxoData.toString(),
      "utxoData does not match",
    );
  }

  assert.strictEqual(
    utxo1.amounts.length,
    utxo2.amounts.length,
    "amounts array length does not match",
  );
  utxo1.amounts.forEach((amount, index) =>
    assert(amount.eq(utxo2.amounts[index]), `amounts[${index}] does not match`),
  );

  assert.strictEqual(
    utxo1.assets.length,
    utxo2.assets.length,
    "assets array length does not match",
  );
  utxo1.assets.forEach((asset, index) =>
    assert.strictEqual(
      asset.toString(),
      utxo2.assets[index].toString(),
      `assets[${index}] does not match`,
    ),
  );

  assert.deepEqual(
    utxo1.assetsCircuit,
    utxo2.assetsCircuit,
    "assetsCircuit does not match",
  );
  assert.strictEqual(
    utxo1.blinding.toString(),
    utxo2.blinding.toString(),
    "blinding does not match",
  );
  assert.strictEqual(utxo1.poolType, utxo2.poolType, "poolType does not match");
  assert.strictEqual(
    utxo1.transactionVersion,
    utxo2.transactionVersion,
    "transactionVersion does not match",
  );
  assert.strictEqual(
    utxo1.isFillingUtxo,
    utxo2.isFillingUtxo,
    "isFillingUtxo does not match",
  );
  assert.strictEqual(
    utxo1.utxoDataHash.toString(),
    utxo2.utxoDataHash.toString(),
    "utxoDataHash does not match",
  );
  assert.strictEqual(utxo1.utxoHash, utxo2.utxoHash, "utxoHash does not match");
}

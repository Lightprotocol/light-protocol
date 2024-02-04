import { assert } from "chai";
import {
  OutUtxo,
  PlaceHolderTData,
  ProgramOutUtxo,
  ProgramUtxo,
  stringifyAssetsToCircuitInput,
} from "../../src/index";
import { Utxo } from "../../src";

export function compareOutUtxos(
  utxo1: OutUtxo | ProgramOutUtxo<PlaceHolderTData>,
  utxo2: OutUtxo | ProgramOutUtxo<PlaceHolderTData>,
): void {
  assert.strictEqual(
    utxo1.owner.toString(),
    utxo2.owner.toString(),
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

  const utxo1AssetsCircuitInput = stringifyAssetsToCircuitInput(utxo1.assets);
  const utxo2AssetsCircuitInput = stringifyAssetsToCircuitInput(utxo2.assets);

  assert.deepEqual(
    utxo1AssetsCircuitInput,
    utxo2AssetsCircuitInput,
    "assetsCircuit does not match",
  );
  assert.strictEqual(
    utxo1.blinding.toString(),
    utxo2.blinding.toString(),
    "blinding does not match",
  );
  assert.strictEqual(utxo1.poolType, utxo2.poolType, "poolType does not match");
  assert.strictEqual(
    utxo1.version,
    utxo2.version,
    "transactionVersion does not match",
  );
  assert.strictEqual(
    utxo1.isFillingUtxo,
    utxo2.isFillingUtxo,
    "isFillingUtxo does not match",
  );
  if ("dataHash" in utxo1 && "dataHash" in utxo2) {
    assert.strictEqual(
      utxo1.dataHash!.toString(),
      utxo2.dataHash!.toString(),
      "utxoDataHash does not match",
    );
  }
  assert.strictEqual(
    utxo1.hash.toString(),
    utxo2.hash.toString(),
    "utxoHash does not match",
  );
}

export function compareUtxos(
  utxo1: Utxo | ProgramUtxo<PlaceHolderTData>,
  utxo2: Utxo | ProgramUtxo<PlaceHolderTData>,
): void {
  assert.strictEqual(
    utxo1.owner.toString(),
    utxo2.owner.toString(),
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

  if ("data" in utxo1 && !("data" in utxo2))
    throw new Error("Utxo type mismatch");
  if ("data" in utxo1 && "data" in utxo2) {
    assert.deepStrictEqual(
      utxo1.data.toString(),
      utxo2.data.toString(),
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

  const utxo1AssetsCircuitInput = stringifyAssetsToCircuitInput(utxo1.assets);
  const utxo2AssetsCircuitInput = stringifyAssetsToCircuitInput(utxo2.assets);

  assert.deepEqual(
    utxo1AssetsCircuitInput,
    utxo2AssetsCircuitInput,
    "assetsCircuit does not match",
  );
  assert.strictEqual(
    utxo1.blinding.toString(),
    utxo2.blinding.toString(),
    "blinding does not match",
  );
  assert.strictEqual(utxo1.poolType, utxo2.poolType, "poolType does not match");

  assert.strictEqual(
    utxo1.version,
    utxo2.version,
    "transactionVersion does not match",
  );
  assert.strictEqual(
    utxo1.isFillingUtxo,
    utxo2.isFillingUtxo,
    "isFillingUtxo does not match",
  );
  if ("dataHash" in utxo1 && "dataHash" in utxo2)
    assert.strictEqual(
      utxo1.dataHash.toString(),
      utxo2.dataHash.toString(),
      "utxoDataHash does not match",
    );
  assert.strictEqual(utxo1.hash, utxo2.hash, "utxoHash does not match");
}

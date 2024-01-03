import { assert } from "chai";
import { SystemProgram } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { it } from "mocha";
import { compareOutUtxos } from "./test-utils/compareUtxos";

import {
  Account,
  hashAndTruncateToCircuit,
  MerkleTreeConfig,
  MINT,
  Provider as LightProvider,
  createOutUtxo,
  outUtxoToBytes,
  outUtxoFromBytes,
  encryptOutUtxo,
  decryptOutUtxo,
  decryptUtxo,
} from "../src";
import { WasmHasher, Hasher } from "@lightprotocol/account.rs";

const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
const seed32 = new Uint8Array(32).fill(1).toString();
let account: Account;
describe("Utxo Functional", () => {
  let hasher: Hasher, lightProvider: LightProvider;
  before(async () => {
    hasher = await WasmHasher.getInstance();
    lightProvider = await LightProvider.loadMock();
    account = Account.createFromSeed(hasher, seed32);
  });

  it("create out utxo", async () => {
    for (let i = 0; i < 100; i++) {
      const outUtxo = createOutUtxo({
        publicKey: account.keypair.publicKey,
        amounts: [new BN(123), new BN(456)],
        assets: [SystemProgram.programId, MINT],
        hasher,
      });
      const assetLookupTable = lightProvider.lookUpTables.assetLookupTable;
      const bytes = await outUtxoToBytes(outUtxo, assetLookupTable);
      const fromBytesOutUtxo = outUtxoFromBytes({
        bytes: Buffer.from(bytes),
        account,
        assetLookupTable,
        hasher,
      });
      compareOutUtxos(fromBytesOutUtxo, outUtxo);

      const compressedBytes = await outUtxoToBytes(
        outUtxo,
        assetLookupTable,
        true,
      );
      const fromBytesCompressedOutUtxo = outUtxoFromBytes({
        bytes: Buffer.from(compressedBytes),
        account,
        assetLookupTable,
        hasher,
        compressed: true,
      });
      compareOutUtxos(fromBytesCompressedOutUtxo, outUtxo);

      const encryptedBytes = await encryptOutUtxo({
        utxo: outUtxo,
        account,
        hasher,
        merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
        assetLookupTable,
      });
      const decryptedUtxo = await decryptOutUtxo({
        encBytes: encryptedBytes,
        account,
        hasher,
        merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
        assetLookupTable,
        aes: true,
        utxoHash: new BN(outUtxo.utxoHash).toArrayLike(Buffer, "be", 32),
      });
      compareOutUtxos(decryptedUtxo.value!, outUtxo);

      const asymOutUtxo = createOutUtxo({
        publicKey: account.keypair.publicKey,
        encryptionPublicKey: account.encryptionKeypair.publicKey,
        amounts: [new BN(123), new BN(456)],
        assets: [SystemProgram.programId, MINT],
        hasher,
      });

      const encryptedBytesNacl = await encryptOutUtxo({
        utxo: asymOutUtxo,
        account,
        hasher,
        merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
        assetLookupTable,
      });
      const decryptedUtxoNacl = await decryptOutUtxo({
        encBytes: encryptedBytesNacl,
        account,
        hasher,
        merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
        assetLookupTable,
        aes: false,
        utxoHash: new BN(asymOutUtxo.utxoHash).toArrayLike(Buffer, "be", 32),
      });
      if (decryptedUtxoNacl.value === null) {
        throw new Error("decrypt nacl failed");
      }
      decryptedUtxoNacl.value!["encryptionPublicKey"] =
        account.encryptionKeypair.publicKey;
      compareOutUtxos(decryptedUtxoNacl.value!, asymOutUtxo);
    }
  });

  it("encryption", async () => {
    const amountFee = "1";
    const amountToken = "2";
    const assetPubkey = MINT;
    const seed32 = new Uint8Array(32).fill(1).toString();
    const inputs = {
      keypair: Account.createFromSeed(hasher, seed32),
      amountFee,
      amountToken,
      assetPubkey,
      assets: [SystemProgram.programId, assetPubkey],
      amounts: [new BN(amountFee), new BN(amountToken)],
      blinding: new BN(new Uint8Array(31).fill(2)),
      index: 1,
    };
    const assetLookupTable = lightProvider.lookUpTables.assetLookupTable;

    const outUtxo = createOutUtxo({
      publicKey: account.keypair.publicKey,
      amounts: inputs.amounts,
      assets: inputs.assets,
      blinding: inputs.blinding,
      hasher,
    });

    // functional
    assert.equal(outUtxo.amounts[0].toString(), amountFee);
    assert.equal(outUtxo.amounts[1].toString(), amountToken);
    assert.equal(
      outUtxo.assets[0].toBase58(),
      SystemProgram.programId.toBase58(),
    );
    assert.equal(outUtxo.assets[1].toBase58(), assetPubkey.toBase58());
    assert.equal(
      outUtxo.assetsCircuit[0].toString(),
      hashAndTruncateToCircuit(SystemProgram.programId.toBytes()).toString(),
    );
    assert.equal(
      outUtxo.assetsCircuit[1].toString(),
      hashAndTruncateToCircuit(assetPubkey.toBytes()).toString(),
    );
    assert.equal(outUtxo.utxoDataHash.toString(), "0");
    assert.equal(outUtxo.poolType.toString(), "0");
    assert.equal(
      outUtxo.verifierAddress.toString(),
      SystemProgram.programId.toString(),
    );
    assert.equal(outUtxo.verifierAddressCircuit.toString(), "0");
    assert.equal(
      outUtxo.utxoHash,
      "10253777838998756860614944496033986881757496982016254670361237551864044449818",
    );

    // toBytes
    const bytes = await outUtxoToBytes(outUtxo, assetLookupTable);
    // fromBytes
    const utxo1 = outUtxoFromBytes({
      hasher,
      account: inputs.keypair,
      bytes: Buffer.from(bytes),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    // Utxo.equal(hasher, utxo0, utxo1);
    compareOutUtxos(utxo1, outUtxo);

    // encrypt
    const encBytes = await encryptOutUtxo({
      utxo: outUtxo,
      hasher,
      account: inputs.keypair,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      assetLookupTable,
      compressed: true,
    });

    // decrypt
    const utxo3 = await decryptOutUtxo({
      hasher,
      encBytes,
      account: inputs.keypair,
      aes: true,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      utxoHash: new BN(outUtxo.utxoHash).toArrayLike(Buffer, "be", 32),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      compressed: true,
    });

    if (utxo3.value) {
      // Utxo.equal(hasher, utxo0, utxo3.value);
      compareOutUtxos(utxo3.value, outUtxo);
    } else {
      throw new Error("decrypt failed");
    }

    const decryptedUtxo = await decryptUtxo(
      encBytes,
      inputs.keypair,
      MerkleTreeConfig.getTransactionMerkleTreePda(),
      true,
      new BN(outUtxo.utxoHash).toArrayLike(Buffer, "be", 32),
      hasher,
      true,
      ["1", "2", "3"],
      inputs.index,
      assetLookupTable,
    );
    assert.equal(decryptedUtxo.value?.amounts[0].toString(), amountFee);
    assert.equal(decryptedUtxo.value?.amounts[1].toString(), amountToken);
    assert.equal(
      decryptedUtxo.value?.assets[0].toBase58(),
      SystemProgram.programId.toBase58(),
    );
    assert.equal(
      decryptedUtxo.value?.assets[1].toBase58(),
      assetPubkey.toBase58(),
    );
    assert.equal(
      decryptedUtxo.value?.assetsCircuit[0].toString(),
      hashAndTruncateToCircuit(SystemProgram.programId.toBytes()).toString(),
    );
    assert.equal(
      decryptedUtxo.value?.assetsCircuit[1].toString(),
      hashAndTruncateToCircuit(assetPubkey.toBytes()).toString(),
    );
    assert.equal(decryptedUtxo.value?.utxoDataHash.toString(), "0");
    assert.equal(decryptedUtxo.value?.poolType.toString(), "0");
    assert.equal(
      decryptedUtxo.value?.verifierAddress.toString(),
      SystemProgram.programId.toString(),
    );
    assert.equal(decryptedUtxo.value?.verifierAddressCircuit.toString(), "0");
    assert.equal(decryptedUtxo.value?.utxoHash, outUtxo.utxoHash);
    assert.equal(
      decryptedUtxo.value?.nullifier,
      "20156180646641338299834793922899381259815381519712122415534487127198510064334",
    );
    assert.deepEqual(decryptedUtxo.value?.merkleProof, ["1", "2", "3"]);
    assert.equal(decryptedUtxo.value?.merkleTreeLeafIndex, inputs.index);

    // encrypting with nacl because this utxo's account does not have an aes secret key since it is instantiated from a public key
    const outUtxoNacl = createOutUtxo({
      publicKey: account.keypair.publicKey,
      encryptionPublicKey: account.encryptionKeypair.publicKey,
      amounts: inputs.amounts,
      assets: inputs.assets,
      blinding: inputs.blinding,
      hasher,
    });

    // encrypt
    const encBytesNacl = await encryptOutUtxo({
      utxo: outUtxoNacl,
      hasher,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      assetLookupTable,
    });

    // decrypt
    const receivingUtxo1Unchecked = await decryptOutUtxo({
      hasher,
      encBytes: encBytesNacl,
      account: inputs.keypair,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      aes: false,
      utxoHash: new BN(outUtxoNacl.utxoHash).toArrayLike(Buffer, "be", 32),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    if (receivingUtxo1Unchecked.value !== null) {
      const decryptedUtxo = receivingUtxo1Unchecked.value;
      decryptedUtxo["encryptionPublicKey"] =
        account.encryptionKeypair.publicKey;
      compareOutUtxos(decryptedUtxo, outUtxoNacl);
    } else {
      throw new Error("decrypt unchecked failed");
    }
  });
});

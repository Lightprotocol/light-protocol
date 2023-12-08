import { assert } from "chai";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { it } from "mocha";
import { compareOutUtxos } from "./test-utils/compare-utxos";

import {
  Account,
  hashAndTruncateToCircuit,
  MERKLE_TREE_SET,
  MerkleTreeConfig,
  MINT,
  Provider as LightProvider,
  createOutUtxo,
  outUtxoToBytes,
  outUtxoFromBytes,
  encryptOutUtxo,
  decryptOutUtxo,
  decryptUtxo,
  createProgramOutUtxo,
  programOutUtxoToBytes,
  BN_1,
  programOutUtxoFromBytes,
  createDataHashWithDefaultHashingSchema,
  stringifyAssetsToCircuitInput,
  createFillingOutUtxo,
  STANDARD_COMPRESSION_PUBLIC_KEY,
} from "../src";
import { LightWasm, WasmFactory } from "@lightprotocol/account.rs";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { IDL as TEST_PSP_IDL } from "./testData/tmp_test_psp";

const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
const seed32 = new Uint8Array(32).fill(1).toString();
let account: Account;
describe("Utxo Functional", () => {
  let lightWasm: LightWasm, lightProvider: LightProvider;
  before(async () => {
    lightWasm = await WasmFactory.getInstance();
    lightProvider = await LightProvider.loadMock();
    account = Account.createFromSeed(lightWasm, seed32);
  });

  it("create out utxo", async () => {
    for (let i = 0; i < 100; i++) {
      const outUtxo = createOutUtxo({
        owner: account.keypair.publicKey,
        amounts: [new BN(123), new BN(456)],
        assets: [SystemProgram.programId, MINT],
        lightWasm,
      });
      const assetLookupTable = lightProvider.lookUpTables.assetLookupTable;
      const bytes = await outUtxoToBytes(outUtxo, assetLookupTable);
      const fromBytesOutUtxo = outUtxoFromBytes({
        bytes: Buffer.from(bytes),
        account,
        assetLookupTable,
        lightWasm,
      });
      compareOutUtxos(fromBytesOutUtxo!, outUtxo);

      const compressedBytes = await outUtxoToBytes(
        outUtxo,
        assetLookupTable,
        true,
      );
      const fromBytesCompressedOutUtxo = outUtxoFromBytes({
        bytes: Buffer.from(compressedBytes),
        account,
        assetLookupTable,
        lightWasm,
        compressed: true,
      });
      compareOutUtxos(fromBytesCompressedOutUtxo!, outUtxo);

      const encryptedBytes = await encryptOutUtxo({
        utxo: outUtxo,
        account,
        lightWasm,
        merkleTreePdaPublicKey: MERKLE_TREE_SET,
        assetLookupTable,
      });
      const decryptedUtxo = await decryptOutUtxo({
        encBytes: encryptedBytes,
        account,
        lightWasm,
        merkleTreePdaPublicKey: MERKLE_TREE_SET,
        assetLookupTable,
        aes: true,
        utxoHash: new BN(outUtxo.hash).toArrayLike(Buffer, "be", 32),
      });
      compareOutUtxos(decryptedUtxo.value!, outUtxo);

      const asymOutUtxo = createOutUtxo({
        owner: account.keypair.publicKey,
        encryptionPublicKey: account.encryptionKeypair.publicKey,
        amounts: [new BN(123), new BN(456)],
        assets: [SystemProgram.programId, MINT],
        lightWasm,
      });
      const expectedPrefix = bs58.encode(
        account.encryptionKeypair.publicKey.slice(0, 4),
      );
      const encryptedBytesNacl = await encryptOutUtxo({
        utxo: asymOutUtxo,
        account,
        lightWasm,
        merkleTreePdaPublicKey: MERKLE_TREE_SET,
        assetLookupTable,
      });
      assert.equal(bs58.encode(encryptedBytesNacl.slice(0, 4)), expectedPrefix);
      const decryptedUtxoNacl = await decryptOutUtxo({
        encBytes: encryptedBytesNacl,
        account,
        lightWasm,
        merkleTreePdaPublicKey: MERKLE_TREE_SET,
        assetLookupTable,
        aes: false,
        utxoHash: new BN(asymOutUtxo.hash).toArrayLike(Buffer, "be", 32),
      });
      if (decryptedUtxoNacl.value === null) {
        throw new Error("decrypt nacl failed");
      }
      decryptedUtxoNacl.value!["encryptionPublicKey"] =
        account.encryptionKeypair.publicKey;
      compareOutUtxos(decryptedUtxoNacl.value!, asymOutUtxo);
    }
  });
  it("parsing for hash", async () => {
    const zeroHash = lightWasm.poseidonHash(["0"]);
    console.log("zero hash: ", zeroHash);
    const oneHash = lightWasm.poseidonHash(["1"]);
    console.log("one hash: ", oneHash);
    const oneBnHash = lightWasm.poseidonHash([new BN(1).toString()]);
    console.log("one bn hash: ", oneBnHash);
  });

  it("Filling public utxo is consistent", async () => {
    const fillingUtxo = createFillingOutUtxo({
      lightWasm,
      publicKey: STANDARD_COMPRESSION_PUBLIC_KEY,
      isPublic: true,
    });
    console.log("filling utxo: ", fillingUtxo.utxoHash);
    const fillingUtxo2 = createFillingOutUtxo({
      lightWasm,
      publicKey: STANDARD_COMPRESSION_PUBLIC_KEY,
      isPublic: true,
    });
    assert.equal(fillingUtxo.utxoHash, fillingUtxo2.utxoHash);
  });

  it("encryption", async () => {
    const amountFee = "1";
    const amountToken = "2";
    const assetPubkey = MINT;
    const seed32 = new Uint8Array(32).fill(1).toString();
    const inputs = {
      keypair: Account.createFromSeed(lightWasm, seed32),
      amountFee,
      amountToken,
      assetPubkey,
      assets: [SystemProgram.programId, assetPubkey],
      amounts: [new BN(amountFee), new BN(amountToken)],
      blinding: new BN(new Uint8Array(31).fill(2)),
      index: 1,
    };
    const assetLookupTable = lightProvider.lookUpTables.assetLookupTable;
    console.log(
      "public key: ",
      inputs.keypair.keypair.publicKey.toArray("be", 32),
    );
    const outUtxo = createOutUtxo({
      owner: account.keypair.publicKey,
      amounts: inputs.amounts,
      assets: inputs.assets,
      blinding: inputs.blinding,
      lightWasm,
    });

    const outUtxoAssetCircuitInput = stringifyAssetsToCircuitInput(
      outUtxo.assets,
    );

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
      hashAndTruncateToCircuit(
        [SystemProgram.programId.toBytes()],
        lightWasm,
      ).toString(),
    );
    assert.equal(
      outUtxo.assetsCircuit[1].toString(),
      hashAndTruncateToCircuit([assetPubkey.toBytes()], lightWasm).toString(),
    );
    if ("data" in outUtxo) throw new Error("dataHash is not 0");
    assert.equal(outUtxo.poolType.toString(), "0");
    assert.equal(
      outUtxo.utxoHash,
      "2544843658061717158156922815997928856082308524175481591473611870665777784472",
    );
    console.log("utxo hash: ", new BN(outUtxo.utxoHash).toArray("be", 32));

    // toBytes
    const bytes = await outUtxoToBytes(outUtxo, assetLookupTable);
    // fromBytes
    const utxo1 = outUtxoFromBytes({
      lightWasm,
      account: inputs.keypair,
      bytes: Buffer.from(bytes),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    compareOutUtxos(utxo1!, outUtxo);

    // encrypt
    const encBytes = await encryptOutUtxo({
      utxo: outUtxo,
      lightWasm,
      account: inputs.keypair,
      merkleTreePdaPublicKey: MERKLE_TREE_SET,
      assetLookupTable,
      compressed: true,
    });

    // decrypt
    const utxo3 = await decryptOutUtxo({
      lightWasm,
      encBytes,
      account: inputs.keypair,
      aes: true,
      merkleTreePdaPublicKey: MERKLE_TREE_SET,
      utxoHash: new BN(outUtxo.hash).toArrayLike(Buffer, "be", 32),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      compressed: true,
    });

    if (utxo3.value) {
      compareOutUtxos(utxo3.value, outUtxo);
    } else {
      throw new Error("decrypt failed");
    }

    const decryptedUtxo = await decryptUtxo(
      encBytes,
      inputs.keypair,
      MERKLE_TREE_SET,
      true,
      new BN(outUtxo.hash).toArrayLike(Buffer, "be", 32),
      lightWasm,
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
    const decryptedUtxoAssetCircuitInput = stringifyAssetsToCircuitInput(
      decryptedUtxo.value!.assets,
    );
    assert.equal(
      decryptedUtxoAssetCircuitInput[0].toString(),
      hashAndTruncateToCircuit(
        [SystemProgram.programId.toBytes()],
        lightWasm,
      ).toString(),
    );
    assert.equal(
      decryptedUtxoAssetCircuitInput[1].toString(),
      hashAndTruncateToCircuit([assetPubkey.toBytes()], lightWasm).toString(),
    );
    assert.equal(decryptedUtxo.value?.poolType.toString(), "0");
    assert.equal(decryptedUtxo.value?.hash.toString(), outUtxo.hash.toString());
    assert.equal(
      decryptedUtxo.value?.nullifier.toString(),
      "10553008000889321107174664413512517396156948704869406505613611769377511828900",
    );
    assert.deepEqual(decryptedUtxo.value?.merkleProof, ["1", "2", "3"]);
    assert.equal(decryptedUtxo.value?.merkleTreeLeafIndex, inputs.index);

    // encrypting with nacl because this utxo's account does not have an aes secret key since it is instantiated from a public key
    const outUtxoNacl = createOutUtxo({
      owner: account.keypair.publicKey,
      encryptionPublicKey: account.encryptionKeypair.publicKey,
      amounts: inputs.amounts,
      assets: inputs.assets,
      blinding: inputs.blinding,
      lightWasm,
    });

    // encrypt
    const encBytesNacl = await encryptOutUtxo({
      utxo: outUtxoNacl,
      lightWasm,
      merkleTreePdaPublicKey: MERKLE_TREE_SET,
      assetLookupTable,
    });

    // decrypt
    const receivingUtxo1Unchecked = await decryptOutUtxo({
      lightWasm,
      encBytes: encBytesNacl,
      account: inputs.keypair,
      merkleTreePdaPublicKey: MERKLE_TREE_SET,
      aes: false,
      utxoHash: new BN(outUtxoNacl.hash).toArrayLike(Buffer, "be", 32),
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

  it("Program utxo to/from bytes", async () => {
    const verifierProgramId = new PublicKey(
      "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
    );

    const seed = bs58.encode(new Uint8Array(32).fill(1));
    const account = Account.createFromSeed(lightWasm, seed);
    const outputUtxo = createProgramOutUtxo({
      lightWasm,
      assets: [SystemProgram.programId],
      amounts: [new BN(1_000_000)],
      data: { releaseSlot: BN_1 },
      dataHash: createDataHashWithDefaultHashingSchema(
        { releaseSlot: BN_1 },
        lightWasm,
      ),
      ownerIdl: TEST_PSP_IDL,
      owner: verifierProgramId,
      type: "utxo",
    });
    const bytes = await programOutUtxoToBytes(
      outputUtxo,
      lightProvider.lookUpTables.assetLookupTable,
      false,
    );

    const utxo1 = programOutUtxoFromBytes({
      lightWasm,
      bytes: Buffer.from(bytes),
      account,
      ownerIdl: TEST_PSP_IDL,
      owner: verifierProgramId,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      type: "utxo",
    });
    compareOutUtxos(utxo1, outputUtxo);
    assert.equal(
      utxo1.owner.toBase58(),
      "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
    );
  });
});

import { assert } from "chai";
import { SystemProgram } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { it } from "mocha";
import { IDL as TEST_PSP_IDL } from "./testData/tmp_test_psp";

import {
  Account,
  BN_1,
  hashAndTruncateToCircuit,
  MerkleTreeConfig,
  MINT,
  Provider as LightProvider,
  createProgramOutUtxo,
  getVerifierProgramId,
  programOutUtxoToBytes,
  programOutUtxoFromBytes,
  encryptProgramOutUtxo,
  decryptProgramOutUtxo,
  decryptProgramUtxo,
} from "../src";
import { WasmFactory, LightWasm } from "@lightprotocol/account.rs";
import { compareOutUtxos } from "./test-utils/compareUtxos";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
const seed32 = new Uint8Array(32).fill(1).toString();
let account: Account;
const pspIdl = TEST_PSP_IDL;
const pspId = getVerifierProgramId(pspIdl);
const utxoData = {
  releaseSlot: BN_1,
};
const utxoName = "utxo";
let createOutUtxoInputs;
describe("Program Utxo Functional", () => {
  let lightWasm: LightWasm, lightProvider: LightProvider;
  before(async () => {
    lightWasm = await WasmFactory.getInstance();
    lightProvider = await LightProvider.loadMock();
    account = Account.createFromSeed(lightWasm, seed32);
    createOutUtxoInputs = {
      publicKey: account.keypair.publicKey,
      amounts: [new BN(123), new BN(456)],
      assets: [SystemProgram.programId, MINT],
      verifierAddress: pspId,
    };
  });

  it("create program out utxo", async () => {
    for (let i = 0; i < 100; i++) {
      const programOutUtxo = createProgramOutUtxo({
        ...createOutUtxoInputs,
        lightWasm,
        pspId,
        utxoName,
        pspIdl,
        utxoData,
      });
      const assetLookupTable = lightProvider.lookUpTables.assetLookupTable;
      const bytes = await programOutUtxoToBytes(
        programOutUtxo,
        assetLookupTable,
      );
      const fromBytesOutUtxo = programOutUtxoFromBytes({
        bytes: Buffer.from(bytes),
        account,
        assetLookupTable,
        lightWasm,
        pspId,
        utxoName,
        pspIdl,
      });
      compareOutUtxos(fromBytesOutUtxo.outUtxo, programOutUtxo.outUtxo);

      const encryptedBytes = await encryptProgramOutUtxo({
        utxo: programOutUtxo,
        account,
        lightWasm,
        merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
        assetLookupTable,
      });
      const decryptedUtxo = await decryptProgramOutUtxo({
        encBytes: encryptedBytes,
        account,
        lightWasm,
        merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
        assetLookupTable,
        aes: true,
        utxoHash: new BN(programOutUtxo.outUtxo.utxoHash).toArrayLike(
          Buffer,
          "be",
          32,
        ),
        pspId,
        utxoName,
        pspIdl,
      });
      compareOutUtxos(decryptedUtxo.value!.outUtxo, programOutUtxo.outUtxo);

      const asymOutUtxoInputs = {
        ...createOutUtxoInputs,
        encryptionPublicKey: account.encryptionKeypair.publicKey,
      };
      const asymOutUtxo = createProgramOutUtxo({
        ...asymOutUtxoInputs,
        lightWasm,
        pspId,
        utxoName,
        pspIdl,
        utxoData,
      });

      const encryptedBytesNacl = await encryptProgramOutUtxo({
        utxo: asymOutUtxo,
        account,
        lightWasm,
        merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
        assetLookupTable,
      });
      const decryptedUtxoNacl = await decryptProgramOutUtxo({
        encBytes: encryptedBytesNacl,
        account,
        lightWasm,
        merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
        assetLookupTable,
        aes: false,
        utxoHash: new BN(asymOutUtxo.outUtxo.utxoHash).toArrayLike(
          Buffer,
          "be",
          32,
        ),
        pspId,
        utxoName,
        pspIdl,
      });
      if (decryptedUtxoNacl.value === null) {
        throw new Error("decrypt nacl failed");
      }
      decryptedUtxoNacl.value!["encryptionPublicKey"] =
        account.encryptionKeypair.publicKey;
      compareOutUtxos(decryptedUtxoNacl.value!.outUtxo, asymOutUtxo.outUtxo);
    }
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
    const createOutUtxoInputs = {
      publicKey: account.keypair.publicKey,
      amounts: [new BN(amountFee), new BN(amountToken)],
      assets: [SystemProgram.programId, MINT],
      verifierAddress: pspId,
      blinding: new BN(new Uint8Array(31).fill(2)),
    };
    const assetLookupTable = lightProvider.lookUpTables.assetLookupTable;

    const programOutUtxo = createProgramOutUtxo({
      ...createOutUtxoInputs,
      lightWasm,
      pspId,
      utxoName,
      pspIdl,
      utxoData,
    });

    // functional
    assert.equal(programOutUtxo.outUtxo.amounts[0].toString(), amountFee);
    assert.equal(programOutUtxo.outUtxo.amounts[1].toString(), amountToken);
    assert.equal(
      programOutUtxo.outUtxo.assets[0].toBase58(),
      SystemProgram.programId.toBase58(),
    );
    assert.equal(
      programOutUtxo.outUtxo.assets[1].toBase58(),
      assetPubkey.toBase58(),
    );
    assert.equal(
      programOutUtxo.outUtxo.assetsCircuit[0].toString(),
      hashAndTruncateToCircuit(SystemProgram.programId.toBytes()).toString(),
    );
    assert.equal(
      programOutUtxo.outUtxo.assetsCircuit[1].toString(),
      hashAndTruncateToCircuit(assetPubkey.toBytes()).toString(),
    );
    assert.equal(
      programOutUtxo.outUtxo.utxoDataHash.toString(),
      lightWasm.poseidonHashString([utxoData.releaseSlot]).toString(),
    );
    assert.equal(programOutUtxo.outUtxo.poolType.toString(), "0");
    assert.equal(
      programOutUtxo.outUtxo.verifierAddress.toString(),
      pspId.toString(),
    );
    assert.equal(
      programOutUtxo.outUtxo.verifierAddressCircuit.toString(),
      hashAndTruncateToCircuit(pspId.toBytes()).toString(),
    );
    assert.equal(
      programOutUtxo.outUtxo.utxoHash,
      "7540600803361927548939886208864149514103175533120235954523664162373509461452",
    );

    // toBytes
    const bytes = await programOutUtxoToBytes(programOutUtxo, assetLookupTable);
    // fromBytes
    const utxo1 = programOutUtxoFromBytes({
      lightWasm,
      account: inputs.keypair,
      bytes: Buffer.from(bytes),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      pspId,
      utxoName,
      pspIdl,
    });
    compareOutUtxos(utxo1.outUtxo, programOutUtxo.outUtxo);

    // encrypt
    const encBytes = await encryptProgramOutUtxo({
      utxo: programOutUtxo,
      lightWasm,
      account: inputs.keypair,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      assetLookupTable,
    });

    // decrypt
    const utxo3 = await decryptProgramOutUtxo({
      lightWasm,
      encBytes,
      account: inputs.keypair,
      aes: true,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      utxoHash: new BN(programOutUtxo.outUtxo.utxoHash).toArrayLike(
        Buffer,
        "be",
        32,
      ),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      pspId,
      utxoName,
      pspIdl,
    });
    if (utxo3.value) {
      compareOutUtxos(utxo3.value.outUtxo, programOutUtxo.outUtxo);
    } else {
      throw new Error("decrypt failed");
    }

    const decryptedUtxo = await decryptProgramUtxo({
      encBytes,
      account: inputs.keypair,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      aes: true,
      utxoHash: new BN(programOutUtxo.outUtxo.utxoHash).toArrayLike(
        Buffer,
        "be",
        32,
      ),
      lightWasm,
      compressed: false,
      merkleProof: ["1", "2", "3"],
      merkleTreeLeafIndex: inputs.index,
      assetLookupTable,
      pspId,
      pspIdl,
      utxoName,
    });

    assert.equal(decryptedUtxo.value?.utxo.amounts[0].toString(), amountFee);
    assert.equal(decryptedUtxo.value?.utxo.amounts[1].toString(), amountToken);
    assert.equal(
      decryptedUtxo.value?.utxo.assets[0].toBase58(),
      SystemProgram.programId.toBase58(),
    );
    assert.equal(
      decryptedUtxo.value?.utxo.assets[1].toBase58(),
      assetPubkey.toBase58(),
    );
    assert.equal(
      decryptedUtxo.value?.utxo.assetsCircuit[0].toString(),
      hashAndTruncateToCircuit(SystemProgram.programId.toBytes()).toString(),
    );
    assert.equal(
      decryptedUtxo.value?.utxo.assetsCircuit[1].toString(),
      hashAndTruncateToCircuit(assetPubkey.toBytes()).toString(),
    );
    assert.equal(
      decryptedUtxo.value?.utxo.utxoDataHash.toString(),
      lightWasm.poseidonHashString([utxoData.releaseSlot]).toString(),
    );
    assert.equal(decryptedUtxo.value?.utxo.poolType.toString(), "0");
    assert.equal(
      decryptedUtxo.value?.utxo.verifierAddress.toString(),
      pspId.toString(),
    );
    assert.equal(
      decryptedUtxo.value?.utxo.verifierAddressCircuit.toString(),
      hashAndTruncateToCircuit(pspId.toBytes()).toString(),
    );
    assert.equal(
      decryptedUtxo.value?.utxo.utxoHash,
      programOutUtxo.outUtxo.utxoHash,
    );
    assert.equal(
      decryptedUtxo.value?.utxo.nullifier,
      "5610794678460418216727726121419735509967163565781683920673823506019224525239",
    );
    assert.deepEqual(decryptedUtxo.value?.utxo.merkleProof, ["1", "2", "3"]);
    assert.equal(decryptedUtxo.value?.utxo.merkleTreeLeafIndex, inputs.index);
    const programOutUtxoNaclInputs = {
      ...createOutUtxoInputs,
      encryptionPublicKey: account.encryptionKeypair.publicKey,
    };
    // encrypting with nacl because this utxo's account does not have an aes secret key since it is instantiated from a public key
    const programOutUtxoNacl = createProgramOutUtxo({
      ...programOutUtxoNaclInputs,
      lightWasm,
      pspId,
      utxoName,
      pspIdl,
      utxoData,
    });

    // encrypt
    const encBytesNacl = await encryptProgramOutUtxo({
      utxo: programOutUtxoNacl,
      lightWasm,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      assetLookupTable,
    });

    // decrypt
    const receivingUtxo1Unchecked = await decryptProgramOutUtxo({
      lightWasm,
      encBytes: encBytesNacl,
      account: inputs.keypair,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      aes: false,
      utxoHash: new BN(programOutUtxoNacl.outUtxo.utxoHash).toArrayLike(
        Buffer,
        "be",
        32,
      ),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      pspId,
      utxoName,
      pspIdl,
    });
    if (receivingUtxo1Unchecked.value !== null) {
      const decryptedProgramUtxo = receivingUtxo1Unchecked.value;
      decryptedProgramUtxo.outUtxo["encryptionPublicKey"] =
        account.encryptionKeypair.publicKey;
      compareOutUtxos(decryptedProgramUtxo.outUtxo, programOutUtxoNacl.outUtxo);
    } else {
      throw new Error("decrypt unchecked failed");
    }
  });
});

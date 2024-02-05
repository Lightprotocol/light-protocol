//@ts-check
import { assert, beforeAll, it } from "vitest";
import { SystemProgram } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { IDL as TEST_PSP_IDL } from "./testData/tmp_test_psp";

import {
  Account,
  BN_1,
  hashAndTruncateToCircuit,
  MERKLE_TREE_SET,
  MINT,
  Provider as LightProvider,
  createProgramOutUtxo,
  getVerifierProgramId,
  programOutUtxoToBytes,
  programOutUtxoFromBytes,
  encryptProgramOutUtxo,
  decryptProgramOutUtxo,
  decryptProgramUtxo,
  getUtxoHash,
  createDataHashWithDefaultHashingSchema,
  stringifyAssetsToCircuitInput,
  getUtxoHashInputs,
} from "../src";
import { WasmFactory, LightWasm } from "@lightprotocol/account.rs";
import { compareOutUtxos } from "./test-utils/compare-utxos";
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
const seed32 = new Uint8Array(32).fill(1).toString();
let account: Account;
const ownerIdl = TEST_PSP_IDL;
const owner = getVerifierProgramId(ownerIdl);
const data = {
  releaseSlot: BN_1,
};
const type = "utxo";
let createOutUtxoInputs;
describe("Program Utxo Functional", () => {
  let lightWasm: LightWasm, lightProvider: LightProvider;
  beforeAll(async () => {
    lightWasm = await WasmFactory.getInstance();
    lightProvider = await LightProvider.loadMock();
    account = Account.createFromSeed(lightWasm, seed32);
    createOutUtxoInputs = {
      owner: account.keypair.publicKey,
      amounts: [new BN(123), new BN(456)],
      assets: [SystemProgram.programId, MINT],
    };
  });

  it("create program out utxo", async () => {
    for (let i = 0; i < 100; i++) {
      const programOutUtxo = createProgramOutUtxo({
        amounts: createOutUtxoInputs.amounts,
        assets: createOutUtxoInputs.assets,
        dataHash: createDataHashWithDefaultHashingSchema(data, lightWasm),
        lightWasm,
        owner,
        type,
        ownerIdl,
        data,
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
        owner,
        type,
        ownerIdl,
      });
      compareOutUtxos(fromBytesOutUtxo, programOutUtxo);

      const encryptedBytes = await encryptProgramOutUtxo({
        utxo: programOutUtxo,
        account,
        lightWasm,
        merkleTreePdaPublicKey: MERKLE_TREE_SET,
        assetLookupTable,
      });
      const decryptedUtxo = await decryptProgramOutUtxo({
        encBytes: encryptedBytes,
        account,
        lightWasm,
        merkleTreePdaPublicKey: MERKLE_TREE_SET,
        assetLookupTable,
        aes: true,
        utxoHash: new BN(programOutUtxo.hash).toArrayLike(Buffer, "be", 32),
        owner,
        type,
        ownerIdl,
      });
      compareOutUtxos(decryptedUtxo.value!, programOutUtxo);

      const asymOutUtxoInputs = {
        ...createOutUtxoInputs,
        encryptionPublicKey: account.encryptionKeypair.publicKey,
      };

      const asymOutUtxo = createProgramOutUtxo({
        ...asymOutUtxoInputs,
        dataHash: createDataHashWithDefaultHashingSchema(data, lightWasm),
        lightWasm,
        owner,
        type,
        ownerIdl,
        data,
      });

      const encryptedBytesNacl = await encryptProgramOutUtxo({
        utxo: asymOutUtxo,
        account,
        lightWasm,
        merkleTreePdaPublicKey: MERKLE_TREE_SET,
        assetLookupTable,
      });
      const decryptedUtxoNacl = await decryptProgramOutUtxo({
        encBytes: encryptedBytesNacl,
        account,
        lightWasm,
        merkleTreePdaPublicKey: MERKLE_TREE_SET,
        assetLookupTable,
        aes: false,
        utxoHash: new BN(asymOutUtxo.hash).toArrayLike(Buffer, "be", 32),
        owner,
        type,
        ownerIdl,
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
      verifierAddress: owner,
      blinding: new BN(new Uint8Array(31).fill(2)),
    };
    const assetLookupTable = lightProvider.lookUpTables.assetLookupTable;

    const programOutUtxo = createProgramOutUtxo({
      amounts: createOutUtxoInputs.amounts,
      assets: createOutUtxoInputs.assets,
      blinding: createOutUtxoInputs.blinding,
      lightWasm,
      owner,
      type,
      ownerIdl,
      data,
      dataHash: lightWasm.poseidonHashBN([data.releaseSlot]),
    });
    const programUtxoAssetsCircuitInput = stringifyAssetsToCircuitInput(
      programOutUtxo.assets,
    );
    // functional
    assert.equal(programOutUtxo.amounts[0].toString(), amountFee);
    assert.equal(programOutUtxo.amounts[1].toString(), amountToken);
    assert.equal(
      programOutUtxo.assets[0].toBase58(),
      SystemProgram.programId.toBase58(),
    );

    assert.equal(programOutUtxo.assets[1].toBase58(), assetPubkey.toBase58());
    assert.equal(
      programUtxoAssetsCircuitInput[0].toString(),
      hashAndTruncateToCircuit(SystemProgram.programId.toBytes()).toString(),
    );
    assert.equal(
      programUtxoAssetsCircuitInput[1].toString(),
      hashAndTruncateToCircuit(assetPubkey.toBytes()).toString(),
    );
    assert.equal(
      programOutUtxo.dataHash.toString(),
      lightWasm.poseidonHashString([data.releaseSlot]).toString(),
    );
    assert.equal(programOutUtxo.poolType.toString(), "0");
    assert.equal(
      programOutUtxo.dataHash.toString(),
      "18586133768512220936620570745912940619677854269274689475585506675881198879027",
    );
    assert.equal(
      programOutUtxo.hash.toString(),
      "3900255133601114289945940646375843533526254833348962507171282032513729686383",
    );

    // toBytes
    const bytes = await programOutUtxoToBytes(programOutUtxo, assetLookupTable);
    // fromBytes

    const utxo1 = programOutUtxoFromBytes({
      lightWasm,
      account: inputs.keypair,
      bytes: Buffer.from(bytes),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      owner,
      type,
      ownerIdl,
    });

    utxo1.dataHash = lightWasm.poseidonHashBN([data.releaseSlot]);

    const programIdCircuitInput = hashAndTruncateToCircuit(
      utxo1.owner.toBytes(),
    );

    const utxoHashInputs = getUtxoHashInputs(
      programIdCircuitInput,
      utxo1.amounts,
      utxo1.assets,
      utxo1.blinding,
      utxo1.poolType,
      utxo1.version,
      utxo1.dataHash,
      utxo1.metaHash,
      utxo1.address,
    );
    utxo1.hash = getUtxoHash(lightWasm, utxoHashInputs);

    compareOutUtxos(utxo1, programOutUtxo);

    // encrypt
    const encBytes = await encryptProgramOutUtxo({
      utxo: programOutUtxo,
      lightWasm,
      account: inputs.keypair,
      merkleTreePdaPublicKey: MERKLE_TREE_SET,
      assetLookupTable,
    });

    // decrypt
    const utxo3 = await decryptProgramOutUtxo({
      lightWasm,
      encBytes,
      account: inputs.keypair,
      aes: true,
      merkleTreePdaPublicKey: MERKLE_TREE_SET,
      utxoHash: new BN(programOutUtxo.hash).toArrayLike(Buffer, "be", 32),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      owner,
      type,
      ownerIdl,
    });

    if (utxo3.value) {
      const programIdCircuitInput = hashAndTruncateToCircuit(
        utxo1.owner.toBytes(),
      );

      const utxoHashInputs = getUtxoHashInputs(
        programIdCircuitInput,
        utxo3.value.amounts,
        utxo3.value.assets,
        utxo3.value.blinding,
        utxo3.value.poolType,
        utxo3.value.version,
        utxo3.value.dataHash,
        utxo3.value.metaHash,
        utxo3.value.address,
      );
      utxo3.value.dataHash = lightWasm.poseidonHashBN([data.releaseSlot]);
      utxo3.value.hash = getUtxoHash(lightWasm, utxoHashInputs);
      compareOutUtxos(utxo3.value, programOutUtxo);
    } else {
      throw new Error("decrypt failed");
    }

    const decryptedUtxo = await decryptProgramUtxo({
      encBytes,
      account: inputs.keypair,
      merkleTreePdaPublicKey: MERKLE_TREE_SET,
      aes: true,
      utxoHash: new BN(programOutUtxo.hash).toArrayLike(Buffer, "be", 32),
      lightWasm,
      compressed: false,
      merkleProof: ["1", "2", "3"],
      merkleTreeLeafIndex: inputs.index,
      assetLookupTable,
      owner,
      ownerIdl,
      type,
    });

    const decryptedUtxoAssetsCircuitInput = stringifyAssetsToCircuitInput(
      decryptedUtxo.value!.assets,
    );

    decryptedUtxo.value!.dataHash = lightWasm.poseidonHashBN([
      data.releaseSlot,
    ]);
    const programIdCircuitInput2 = hashAndTruncateToCircuit(
      utxo1.owner.toBytes(),
    );

    const utxoHashInputs2 = getUtxoHashInputs(
      programIdCircuitInput2,
      programOutUtxo.amounts,
      programOutUtxo.assets,
      programOutUtxo.blinding,
      programOutUtxo.poolType,
      programOutUtxo.version,
      programOutUtxo.dataHash,
      programOutUtxo.metaHash,
      programOutUtxo.address,
    );
    assert.equal(
      decryptedUtxo.value!.hash.toString(),
      getUtxoHash(lightWasm, utxoHashInputs2).toString(),
    );
    decryptedUtxo.value!.hash = getUtxoHash(lightWasm, utxoHashInputs2);

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
      decryptedUtxoAssetsCircuitInput[0].toString(),
      hashAndTruncateToCircuit(SystemProgram.programId.toBytes()).toString(),
    );
    assert.equal(
      decryptedUtxoAssetsCircuitInput[1].toString(),
      hashAndTruncateToCircuit(assetPubkey.toBytes()).toString(),
    );
    assert.equal(
      decryptedUtxo.value?.dataHash.toString(),
      lightWasm.poseidonHashString([data.releaseSlot]).toString(),
    );
    assert.equal(decryptedUtxo.value?.poolType.toString(), "0");

    assert.equal(
      decryptedUtxo.value?.hash.toString(),
      programOutUtxo.hash.toString(),
    );

    assert.equal(
      decryptedUtxo.value?.nullifier.toString(),
      "7348232893700449159949977118836554677452891838957421208915931061103447385461",
    );
    assert.deepEqual(decryptedUtxo.value?.merkleProof, ["1", "2", "3"]);
    assert.equal(decryptedUtxo.value?.merkleTreeLeafIndex, inputs.index);
    const programOutUtxoNaclInputs = {
      ...createOutUtxoInputs,
      encryptionPublicKey: account.encryptionKeypair.publicKey,
    };
    const dataHash = createDataHashWithDefaultHashingSchema(data, lightWasm);
    // encrypting with nacl because this utxo's account does not have an aes secret key since it is instantiated from a public key
    const programOutUtxoNacl = createProgramOutUtxo({
      ...programOutUtxoNaclInputs,
      lightWasm,
      owner,
      type,
      ownerIdl,
      data,
      dataHash,
    });

    // encrypt
    const encBytesNacl = await encryptProgramOutUtxo({
      utxo: programOutUtxoNacl,
      lightWasm,
      merkleTreePdaPublicKey: MERKLE_TREE_SET,
      assetLookupTable,
    });

    // decrypt
    const receivingUtxo1Unchecked = await decryptProgramOutUtxo({
      lightWasm,
      encBytes: encBytesNacl,
      account: inputs.keypair,
      merkleTreePdaPublicKey: MERKLE_TREE_SET,
      aes: false,
      utxoHash: new BN(programOutUtxoNacl.hash).toArrayLike(Buffer, "be", 32),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      owner,
      type,
      ownerIdl,
    });

    if (receivingUtxo1Unchecked.value !== null) {
      const decryptedProgramUtxo = receivingUtxo1Unchecked.value;
      decryptedProgramUtxo["encryptionPublicKey"] =
        account.encryptionKeypair.publicKey;
      compareOutUtxos(decryptedProgramUtxo, programOutUtxoNacl);
    } else {
      throw new Error("decrypt unchecked failed");
    }
  });
});

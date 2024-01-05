import { assert, expect } from "chai";
import {
  Keypair as SolanaKeypair,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { it } from "mocha";
import { IDL as TEST_PSP_IDL } from "./testData/tmp_test_psp_legacy";

import {
  Account,
  BN_1,
  BN_2,
  createAccountObject,
  FIELD_SIZE,
  hashAndTruncateToCircuit,
  MerkleTreeConfig,
  MINT,
  Provider as LightProvider,
  Utxo,
  UtxoError,
  UtxoErrorCode,
  lightPsp4in4outAppStorageId,
  CreateUtxoErrorCode,
} from "../src";
import { LightWasm, WasmFactory } from "@lightprotocol/account.rs";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

describe("Utxo Functional", () => {
  let lightWasm: LightWasm, lightProvider: LightProvider;
  before(async () => {
    lightWasm = await WasmFactory.getInstance();
    // TODO: make fee mandatory
    lightProvider = await LightProvider.loadMock();
  });

  it("rnd utxo functional loop 100", async () => {
    for (let i = 0; i < 100; i++) {
      // try basic tests for rnd empty utxo
      const utxo4Account = Account.random(lightWasm);
      const utxo4 = new Utxo({
        lightWasm,
        amounts: [new BN(123)],
        publicKey: utxo4Account.keypair.publicKey,
        appDataHash: new BN(lightPsp4in4outAppStorageId.toBuffer()),
        includeAppData: false,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      });

      // toBytesProvider
      const bytes4 = await utxo4.toBytes();

      // fromBytes
      const utxo40 = Utxo.fromBytes({
        lightWasm,
        bytes: bytes4,
        index: 0,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      });
      Utxo.equal(utxo4, utxo40, lightWasm);

      // toBytes
      const bytes4Compressed = await utxo4.toBytes(true);

      // fromBytes
      const utxo40Compressed = Utxo.fromBytes({
        lightWasm,
        account: utxo4Account,
        bytes: bytes4Compressed,
        index: 0,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      });
      Utxo.equal(utxo4, utxo40Compressed, lightWasm);

      // encrypt
      const encBytes4 = await utxo4.encrypt({
        lightWasm,
        account: utxo4Account,
        merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      });
      const encBytes41 = await utxo4.encrypt({
        lightWasm,
        account: utxo4Account,
        merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      });
      assert.equal(
        encBytes4.slice(4).toString(),
        encBytes41.slice(4).toString(),
      );

      // decrypt checked
      const utxo41 = await Utxo.decryptUnchecked({
        lightWasm,
        encBytes: encBytes4,
        account: utxo4Account,
        aes: true,
        index: 0,
        merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
        commitment: new BN(utxo4.getCommitment(lightWasm)).toArrayLike(
          Buffer,
          "be",
          32,
        ),
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        merkleProof: [],
      });

      if (utxo41.value) {
        Utxo.equal(utxo4, utxo41.value, lightWasm);
      } else {
        throw new Error(`decrypt failed: ${utxo41.error?.toString()}`);
      }

      // decrypt unchecked
      const utxo41u = await Utxo.decryptUnchecked({
        lightWasm,
        encBytes: encBytes4,
        account: utxo4Account,
        aes: true,
        index: 0,
        merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
        commitment: new BN(utxo4.getCommitment(lightWasm)).toBuffer("be", 32),
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        merkleProof: [],
      });

      if (utxo41u.value !== null) {
        Utxo.equal(utxo4, utxo41u.value, lightWasm);
      } else {
        throw new Error("decrypt unchecked failed");
      }
    }
  });

  it("toString", async () => {
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

    const utxo0 = new Utxo({
      lightWasm,
      assets: inputs.assets,
      amounts: inputs.amounts,
      publicKey: inputs.keypair.keypair.publicKey,
      encryptionPublicKey: inputs.keypair.encryptionKeypair.publicKey,
      blinding: inputs.blinding,
      index: inputs.index,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    const string = await utxo0.toString();
    const utxo1 = Utxo.fromString(
      string,
      lightProvider.lookUpTables.assetLookupTable,
        lightWasm
    );
    // cannot compute nullifier in utxo1 because no privkey is serialized with toString()
    Utxo.equal(utxo0, utxo1, lightWasm, true);
  });

  it("encryption", async () => {
    const amountFee = "1";
    const amountToken = "2";
    const assetPubkey = MINT;
    const seed32 = new Uint8Array(32).fill(1).toString();
    const inputs = {
      account: Account.createFromSeed(lightWasm, seed32),
      amountFee,
      amountToken,
      assetPubkey,
      assets: [SystemProgram.programId, assetPubkey],
      amounts: [new BN(amountFee), new BN(amountToken)],
      blinding: new BN(new Uint8Array(31).fill(2)),
      index: 1,
    };

    const utxo0 = new Utxo({
      lightWasm,
      assets: inputs.assets,
      amounts: inputs.amounts,
      publicKey: inputs.account.keypair.publicKey,
      blinding: inputs.blinding,
      index: inputs.index,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    // functional
    assert.equal(utxo0.amounts[0].toString(), amountFee);
    assert.equal(utxo0.amounts[1].toString(), amountToken);
    assert.equal(
      utxo0.assets[0].toBase58(),
      SystemProgram.programId.toBase58(),
    );
    assert.equal(utxo0.assets[1].toBase58(), assetPubkey.toBase58());
    assert.equal(
      utxo0.assetsCircuit[0].toString(),
      hashAndTruncateToCircuit(SystemProgram.programId.toBytes()).toString(),
    );
    assert.equal(
      utxo0.assetsCircuit[1].toString(),
      hashAndTruncateToCircuit(assetPubkey.toBytes()).toString(),
    );
    assert.equal(utxo0.appDataHash.toString(), "0");
    assert.equal(utxo0.poolType.toString(), "0");
    assert.equal(
      utxo0.verifierAddress.toString(),
      SystemProgram.programId.toString(),
    );
    assert.equal(utxo0.verifierAddressCircuit.toString(), "0");
    assert.equal(
      utxo0.getCommitment(lightWasm)?.toString(),
      "10253777838998756860614944496033986881757496982016254670361237551864044449818",
    );

    assert.equal(
      utxo0.getNullifier({ lightWasm, account: inputs.account })?.toString(),
      "20156180646641338299834793922899381259815381519712122415534487127198510064334",
    );

    // toBytes
    const bytes = await utxo0.toBytes();
    // fromBytes
    const utxo1 = Utxo.fromBytes({
      lightWasm,
      account: inputs.account,
      bytes,
      index: inputs.index,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    Utxo.equal(utxo0, utxo1, lightWasm);

    // encrypt
    const encBytes = await utxo1.encrypt({
      lightWasm,
      account: inputs.account,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
    });

    // decrypt
    const utxo3 = await Utxo.decryptUnchecked({
      lightWasm,
      encBytes,
      account: inputs.account,
      aes: true,
      index: inputs.index,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      commitment: new BN(utxo1.getCommitment(lightWasm)).toArrayLike(
        Buffer,
        "be",
        32,
      ),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      merkleProof: [],
    });
    if (utxo3.value) {
      Utxo.equal(utxo0, utxo3.value, lightWasm);
    } else {
      throw new Error("decrypt failed");
    }

    const publicKey = inputs.account.getPublicKey();
    // encrypting with nacl because this utxo's account does not have an aes secret key since it is instantiated from a public key
    const receivingUtxo = new Utxo({
      lightWasm,
      assets: inputs.assets,
      amounts: inputs.amounts,
      publicKey: Account.fromPubkey(publicKey, lightWasm).keypair.publicKey,
      encryptionPublicKey: Account.fromPubkey(publicKey, lightWasm)
        .encryptionKeypair.publicKey,
      blinding: inputs.blinding,
      index: inputs.index,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    // encrypt
    const encBytesNacl = await receivingUtxo.encrypt({
      lightWasm,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
    });

    // decrypt
    const receivingUtxo1Unchecked = await Utxo.decryptUnchecked({
      lightWasm,
      encBytes: encBytesNacl,
      account: inputs.account,
      index: inputs.index,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      aes: false,
      commitment: new BN(receivingUtxo.getCommitment(lightWasm)).toArrayLike(
        Buffer,
        "be",
        32,
      ),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      merkleProof: [],
    });
    if (receivingUtxo1Unchecked.value !== null) {
      Utxo.equal(receivingUtxo, receivingUtxo1Unchecked.value, lightWasm, true);
    } else {
      throw new Error("decrypt unchecked failed");
    }

    const receivingUtxoNoAes = new Utxo({
      lightWasm,
      assets: inputs.assets,
      amounts: inputs.amounts,
      publicKey: Account.fromPubkey(publicKey, lightWasm).keypair.publicKey,
      encryptionPublicKey: Account.fromPubkey(publicKey, lightWasm)
        .encryptionKeypair.publicKey,
      blinding: inputs.blinding,
      index: inputs.index,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
  });

  it("Program utxo to/from bytes", async () => {
    const verifierProgramId = new PublicKey(
      "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
    );

    const seed = bs58.encode(new Uint8Array(32).fill(1));
    const account = Account.createFromSeed(lightWasm, seed);
    const outputUtxo = new Utxo({
      lightWasm,
      assets: [SystemProgram.programId],
      publicKey: account.keypair.publicKey,
      amounts: [new BN(1_000_000)],
      appData: { releaseSlot: BN_1 },
      appDataIdl: TEST_PSP_IDL,
      verifierAddress: verifierProgramId,
      index: 0,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    const bytes = await outputUtxo.toBytes();

    const utxo1 = Utxo.fromBytes({
      lightWasm,
      bytes,
      index: 0,
      account,
      appDataIdl: TEST_PSP_IDL,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    Utxo.equal(outputUtxo, utxo1, lightWasm);
    assert.equal(
      utxo1.verifierAddress.toBase58(),
      "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
    );
  });
  it("Pick app data from utxo data", () => {
    const data = createAccountObject(
      {
        releaseSlot: 1,
        rndOtherStuff: { s: 2342 },
        o: [2, 2, BN_2],
      },
      TEST_PSP_IDL.accounts,
      "utxoAppData",
    );
    assert.equal(data.releaseSlot, 1);
    assert.equal(data.currentSlot, undefined);
    assert.equal(data.rndOtherStuff, undefined);
    assert.equal(data.o, undefined);

    expect(() => {
      createAccountObject(
        { rndOtherStuff: { s: 2342 }, o: [2, 2, BN_2] },
        TEST_PSP_IDL.accounts,
        "utxoAppData",
      );
    }).to.throw(Error);
  });
});

describe("Utxo Errors", () => {
  const seed32 = bs58.encode(new Uint8Array(32).fill(1));

  let lightWasm: LightWasm, inputs: any, keypair: Account;

  const amountFee = "1";
  const amountToken = "2";
  const assetPubkey = MINT;
  let lightProvider: LightProvider;
  before(async () => {
    lightProvider = await LightProvider.loadMock();
    lightWasm = await WasmFactory.getInstance();
    keypair = Account.createFromSeed(lightWasm, seed32);
    inputs = {
      keypair: Account.createFromSeed(lightWasm, seed32),
      amountFee,
      amountToken,
      assetPubkey,
      assets: [SystemProgram.programId, assetPubkey],
      amounts: [new BN(amountFee), new BN(amountToken)],
      blinding: new BN(new Uint8Array(31).fill(2)),
    };
  });

  it("get nullifier without index", async () => {
    const publicKey = keypair.getPublicKey();
    const account = Account.fromPubkey(publicKey, lightWasm);
    const pubkeyUtxo = new Utxo({
      lightWasm,
      amounts: [BN_1],
      publicKey: account.keypair.publicKey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    expect(() => {
      pubkeyUtxo.getNullifier({ lightWasm, account });
    })
      .throw(UtxoError)
      .include({
        code: UtxoErrorCode.INDEX_NOT_PROVIDED,
        functionName: "getNullifier",
      });
  });

  it("get nullifier without private key", async () => {
    const publicKey = keypair.getPublicKey();

    const account = Account.fromPubkey(publicKey, lightWasm);
    const pubkeyUtxo = new Utxo({
      lightWasm,
      amounts: [BN_1],
      publicKey: account.keypair.publicKey,
      index: 1,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    expect(() => {
      // @ts-ignore
      pubkeyUtxo.getNullifier({ lightWasm });
    })
      .throw(UtxoError)
      .include({
        code: CreateUtxoErrorCode.ACCOUNT_UNDEFINED,
        functionName: "getNullifier",
      });
  });

  it("INVALID_ASSET_OR_AMOUNTS_LENGTH", () => {
    expect(() => {
      new Utxo({
        lightWasm,
        assets: [inputs.assets[1]],
        amounts: inputs.amounts,
        publicKey: inputs.keypair.pubkey,
        blinding: inputs.blinding,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      });
    })
      .to.throw(UtxoError)
      .to.include({
        code: UtxoErrorCode.INVALID_ASSET_OR_AMOUNTS_LENGTH,
        codeMessage: "Length mismatch assets: 1 != amounts: 2",
      });
  });

  it("EXCEEDED_MAX_ASSETS", () => {
    expect(() => {
      new Utxo({
        lightWasm,
        assets: [MINT, MINT, MINT],
        amounts: [BN_1, BN_1, BN_1],
        publicKey: inputs.keypair.pubkey,
        blinding: inputs.blinding,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      });
    })
      .to.throw(UtxoError)
      .to.include({
        code: UtxoErrorCode.EXCEEDED_MAX_ASSETS,
        codeMessage: "assets.length 3 > N_ASSETS 2",
      });
  });

  it("NEGATIVE_AMOUNT", () => {
    expect(() => {
      new Utxo({
        lightWasm,
        assets: inputs.assets,
        amounts: [inputs.amounts[0], new BN(-1)],
        publicKey: inputs.keypair.pubkey,
        blinding: inputs.blinding,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      });
    })
      .to.throw(UtxoError)
      .to.include({
        code: UtxoErrorCode.NEGATIVE_AMOUNT,
        codeMessage: "amount cannot be negative, amounts[1] = -1",
      });
  });

  it("APP_DATA_IDL_UNDEFINED", () => {
    expect(() => {
      new Utxo({
        lightWasm,
        assets: inputs.assets,
        amounts: inputs.amounts,
        publicKey: inputs.keypair.keypair.publicKey,
        blinding: inputs.blinding,
        appData: new Array(32).fill(1),
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      });
    })
      .to.throw(UtxoError)
      .to.include({
        code: UtxoErrorCode.APP_DATA_IDL_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("ASSET_NOT_FOUND", async () => {
    expect(() => {
      new Utxo({
        lightWasm,
        assets: [SystemProgram.programId, SolanaKeypair.generate().publicKey],
        amounts: inputs.amounts,
        publicKey: inputs.keypair.pubkey,
        blinding: inputs.blinding,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      });
    })
      .to.throw(UtxoError)
      .to.include({
        code: UtxoErrorCode.ASSET_NOT_FOUND,
        functionName: "constructor",
      });
  });

  it("BLINDING_EXCEEDS_FIELD_SIZE", async () => {
    expect(() => {
      new Utxo({
        lightWasm,
        assets: [SystemProgram.programId, SolanaKeypair.generate().publicKey],
        amounts: inputs.amounts,
        publicKey: inputs.keypair.pubkey,
        blinding: new BN(FIELD_SIZE),
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      });
    })
      .to.throw(UtxoError)
      .to.include({
        code: UtxoErrorCode.BLINDING_EXCEEDS_FIELD_SIZE,
        functionName: "constructor",
      });
  });
});
import { assert, expect } from "chai";
import {
  Keypair as SolanaKeypair,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { it } from "mocha";
import { IDL as TEST_PSP_IDL } from "./testData/tmp_test_psp";

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
  UTXO_PREFIX_LENGTH,
  UtxoError,
  UtxoErrorCode,
  verifierProgramTwoProgramId,
  CreateUtxoErrorCode,
} from "../src";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { randomBytes } from "tweetnacl";

const circomlibjs = require("circomlibjs");
const { buildPoseidonOpt } = circomlibjs;
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

describe("Utxo Functional", () => {
  let poseidon: any, lightProvider: LightProvider;
  before(async () => {
    poseidon = await buildPoseidonOpt();
    // TODO: make fee mandatory
    lightProvider = await LightProvider.loadMock();
  });

  it("rnd utxo functional loop 100", async () => {
    for (let i = 0; i < 100; i++) {
      // try basic tests for rnd empty utxo
      const utxo4Account = new Account({ poseidon });
      const utxo4 = new Utxo({
        poseidon,
        amounts: [new BN(123)],
        publicKey: utxo4Account.pubkey,
        appDataHash: new BN(verifierProgramTwoProgramId.toBuffer()),
        includeAppData: false,
        verifierAddress: new PublicKey(
          lightProvider.lookUpTables.verifierProgramLookupTable[1],
        ),
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
      });

      // toBytesProvider
      const bytes4 = await utxo4.toBytes();

      // fromBytes
      const utxo40 = Utxo.fromBytes({
        poseidon,
        bytes: bytes4,
        index: 0,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
      });
      Utxo.equal(poseidon, utxo4, utxo40);

      // toBytes
      const bytes4Compressed = await utxo4.toBytes(true);

      // fromBytes
      const utxo40Compressed = Utxo.fromBytes({
        poseidon,
        account: utxo4Account,
        bytes: bytes4Compressed,
        index: 0,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
      });
      Utxo.equal(poseidon, utxo4, utxo40Compressed);

      // encrypt
      const encBytes4 = await utxo4.encrypt({
        poseidon,
        account: utxo4Account,
        merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      });
      const encBytes41 = await utxo4.encrypt({
        poseidon,
        account: utxo4Account,
        merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      });
      assert.equal(encBytes4.toString(), encBytes41.toString());

      // decrypt checked
      const utxo41 = await Utxo.decrypt({
        poseidon,
        encBytes: encBytes4,
        account: utxo4Account,
        aes: true,
        index: 0,
        merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
        commitment: new BN(utxo4.getCommitment(poseidon)).toArrayLike(
          Buffer,
          "le",
          32,
        ),
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
      });

      if (utxo41.value) {
        Utxo.equal(poseidon, utxo4, utxo41.value);
      } else {
        throw new Error(`decrypt failed: ${utxo41.error?.toString()}`);
      }

      // decrypt unchecked
      const utxo41u = await Utxo.decryptUnchecked({
        poseidon,
        encBytes: encBytes4,
        account: utxo4Account,
        aes: true,
        index: 0,
        merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
        commitment: new BN(utxo4.getCommitment(poseidon)).toBuffer("le", 32),
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
      });

      if (utxo41u.value !== null) {
        Utxo.equal(poseidon, utxo4, utxo41u.value);
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
      keypair: new Account({ poseidon, seed: seed32 }),
      amountFee,
      amountToken,
      assetPubkey,
      assets: [SystemProgram.programId, assetPubkey],
      amounts: [new BN(amountFee), new BN(amountToken)],
      blinding: new BN(new Uint8Array(31).fill(2)),
      index: 1,
    };

    const utxo0 = new Utxo({
      poseidon,
      assets: inputs.assets,
      amounts: inputs.amounts,
      publicKey: inputs.keypair.pubkey,
      encryptionPublicKey: inputs.keypair.encryptionKeypair.publicKey,
      blinding: inputs.blinding,
      index: inputs.index,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    const string = await utxo0.toString();
    const utxo1 = Utxo.fromString(
      string,
      poseidon,
      lightProvider.lookUpTables.assetLookupTable,
      lightProvider.lookUpTables.verifierProgramLookupTable,
    );
    // cannot compute nullifier in utxo1 because no privkey is serialized with toString()
    Utxo.equal(poseidon, utxo0, utxo1, true);
  });

  it("encryption", async () => {
    const amountFee = "1";
    const amountToken = "2";
    const assetPubkey = MINT;
    const seed32 = new Uint8Array(32).fill(1).toString();
    const inputs = {
      keypair: new Account({ poseidon, seed: seed32 }),
      amountFee,
      amountToken,
      assetPubkey,
      assets: [SystemProgram.programId, assetPubkey],
      amounts: [new BN(amountFee), new BN(amountToken)],
      blinding: new BN(new Uint8Array(31).fill(2)),
      index: 1,
    };

    const utxo0 = new Utxo({
      poseidon,
      assets: inputs.assets,
      amounts: inputs.amounts,
      publicKey: inputs.keypair.pubkey,
      blinding: inputs.blinding,
      index: inputs.index,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
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
      utxo0.getCommitment(poseidon)?.toString(),
      "8291567517196483063353958025601041123319055074768288393371971758484371715486",
    );

    assert.equal(
      utxo0.getNullifier({ poseidon, account: inputs.keypair })?.toString(),
      "6203060337570741528902613554275892537213176828384528961609701446906034353029",
    );

    // toBytes
    const bytes = await utxo0.toBytes();
    // fromBytes
    const utxo1 = Utxo.fromBytes({
      poseidon,
      account: inputs.keypair,
      bytes,
      index: inputs.index,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    Utxo.equal(poseidon, utxo0, utxo1);

    // encrypt
    const encBytes = await utxo1.encrypt({
      poseidon,
      account: inputs.keypair,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
    });

    // decrypt
    const utxo3 = await Utxo.decrypt({
      poseidon,
      encBytes,
      account: inputs.keypair,
      aes: true,
      index: inputs.index,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      commitment: new BN(utxo1.getCommitment(poseidon)).toArrayLike(
        Buffer,
        "le",
        32,
      ),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    if (utxo3.value) {
      Utxo.equal(poseidon, utxo0, utxo3.value);
    } else {
      throw new Error("decrypt failed");
    }

    const publicKey = inputs.keypair.getPublicKey();
    // encrypting with nacl because this utxo's account does not have an aes secret key since it is instantiated from a public key
    const receivingUtxo = new Utxo({
      poseidon,
      assets: inputs.assets,
      amounts: inputs.amounts,
      publicKey: Account.fromPubkey(publicKey, poseidon).pubkey,
      encryptionPublicKey: Account.fromPubkey(publicKey, poseidon)
        .encryptionKeypair.publicKey,
      blinding: inputs.blinding,
      index: inputs.index,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    // encrypt
    const encBytesNacl = await receivingUtxo.encrypt({
      poseidon,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
    });

    // decrypt
    const receivingUtxo1Unchecked = await Utxo.decryptUnchecked({
      poseidon,
      encBytes: encBytesNacl,
      account: inputs.keypair,
      index: inputs.index,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      aes: false,
      commitment: new BN(receivingUtxo.getCommitment(poseidon)).toArrayLike(
        Buffer,
        "le",
        32,
      ),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    if (receivingUtxo1Unchecked.value !== null) {
      Utxo.equal(poseidon, receivingUtxo, receivingUtxo1Unchecked.value, true);
    } else {
      throw new Error("decrypt unchecked failed");
    }

    const receivingUtxoNoAes = new Utxo({
      poseidon,
      assets: inputs.assets,
      amounts: inputs.amounts,
      publicKey: Account.fromPubkey(publicKey, poseidon).pubkey,
      encryptionPublicKey: Account.fromPubkey(publicKey, poseidon)
        .encryptionKeypair.publicKey,
      blinding: inputs.blinding,
      index: inputs.index,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    // encrypt
    const encBytesNaclNoAes = await receivingUtxoNoAes.encrypt({
      poseidon,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
    });

    // decrypt
    const receivingUtxo1NoAes = await Utxo.decrypt({
      poseidon,
      encBytes: encBytesNaclNoAes,
      account: inputs.keypair,
      index: inputs.index,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      aes: false,
      commitment: new BN(receivingUtxoNoAes.getCommitment(poseidon)).toBuffer(
        "le",
        32,
      ),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    if (receivingUtxo1NoAes.error) {
      throw new Error("decrypt checked failed " + receivingUtxo1NoAes.error);
    }
  });

  it("Program utxo to/from bytes", async () => {
    const verifierProgramId = new PublicKey(
      "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
    );

    const account = new Account({
      poseidon,
      seed: bs58.encode(new Uint8Array(32).fill(1)),
    });
    const outputUtxo = new Utxo({
      poseidon,
      assets: [SystemProgram.programId],
      publicKey: account.pubkey,
      amounts: [new BN(1_000_000)],
      appData: { releaseSlot: BN_1 },
      appDataIdl: TEST_PSP_IDL,
      verifierAddress: verifierProgramId,
      index: 0,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    const bytes = await outputUtxo.toBytes();

    const utxo1 = Utxo.fromBytes({
      poseidon,
      bytes,
      index: 0,
      account,
      appDataIdl: TEST_PSP_IDL,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    Utxo.equal(poseidon, outputUtxo, utxo1);
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

  let poseidon: any, inputs: any, keypair: Account;

  const amountFee = "1";
  const amountToken = "2";
  const assetPubkey = MINT;
  let lightProvider: LightProvider;
  before(async () => {
    lightProvider = await LightProvider.loadMock();
    poseidon = await buildPoseidonOpt();
    keypair = new Account({ poseidon: poseidon, seed: seed32 });
    inputs = {
      keypair: new Account({ poseidon, seed: seed32 }),
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
    const account = Account.fromPubkey(publicKey, poseidon);
    const pubkeyUtxo = new Utxo({
      poseidon,
      amounts: [BN_1],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    expect(() => {
      pubkeyUtxo.getNullifier({ poseidon, account });
    })
      .throw(UtxoError)
      .include({
        code: UtxoErrorCode.INDEX_NOT_PROVIDED,
        functionName: "getNullifier",
      });
  });

  it("get nullifier without private key", async () => {
    const publicKey = keypair.getPublicKey();

    const account = Account.fromPubkey(publicKey, poseidon);
    const pubkeyUtxo = new Utxo({
      poseidon,
      amounts: [BN_1],
      publicKey: account.pubkey,
      index: 1,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    expect(() => {
      // @ts-ignore
      pubkeyUtxo.getNullifier({ poseidon });
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
        poseidon,
        assets: [inputs.assets[1]],
        amounts: inputs.amounts,
        publicKey: inputs.keypair.pubkey,
        blinding: inputs.blinding,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
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
        poseidon,
        assets: [MINT, MINT, MINT],
        amounts: [BN_1, BN_1, BN_1],
        publicKey: inputs.keypair.pubkey,
        blinding: inputs.blinding,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
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
        poseidon,
        assets: inputs.assets,
        amounts: [inputs.amounts[0], new BN(-1)],
        publicKey: inputs.keypair.pubkey,
        blinding: inputs.blinding,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
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
        poseidon,
        assets: inputs.assets,
        amounts: inputs.amounts,
        publicKey: inputs.keypair.pubkey,
        blinding: inputs.blinding,
        appData: new Array(32).fill(1),
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
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
        poseidon,
        assets: [SystemProgram.programId, SolanaKeypair.generate().publicKey],
        amounts: inputs.amounts,
        publicKey: inputs.keypair.pubkey,
        blinding: inputs.blinding,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
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
        poseidon,
        assets: [SystemProgram.programId, SolanaKeypair.generate().publicKey],
        amounts: inputs.amounts,
        publicKey: inputs.keypair.pubkey,
        blinding: new BN(FIELD_SIZE),
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
      });
    })
      .to.throw(UtxoError)
      .to.include({
        code: UtxoErrorCode.BLINDING_EXCEEDS_FIELD_SIZE,
        functionName: "constructor",
      });
  });

  it("decryption fails with checkPrefixHash on prefix generated from random bytes", async () => {
    const publicKey = keypair.getPublicKey();
    const receivingUtxo = new Utxo({
      poseidon,
      assets: inputs.assets,
      amounts: inputs.amounts,
      publicKey: Account.fromPubkey(publicKey, poseidon).pubkey,
      encryptionPublicKey: Account.fromPubkey(publicKey, poseidon)
        .encryptionKeypair.publicKey,
      blinding: inputs.blinding,
      index: inputs.index,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    // encrypt
    const encBytesNacl = await receivingUtxo.encrypt({
      poseidon,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
    });

    // decrypt
    const receivingUtxo1 = await Utxo.decrypt({
      poseidon,
      encBytes: encBytesNacl,
      account: inputs.keypair,
      index: inputs.index,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      aes: false,
      commitment: new BN(receivingUtxo.getCommitment(poseidon)).toBuffer(
        "le",
        32,
      ),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    if (receivingUtxo1.value) {
      throw new Error("decrypt succeeded");
    }
  });
});

describe("Utxo Benchmark", () => {
  let poseidon: any, lightProvider: LightProvider;
  before(async () => {
    poseidon = await buildPoseidonOpt();
    lightProvider = await LightProvider.loadMock();
  });

  it("Decrypting 256k UTXOs w/checkPrefixHash", async () => {
    const utxoTestAccount = new Account({ poseidon });
    const testUtxo = new Utxo({
      poseidon,
      amounts: [new BN(123)],
      publicKey: utxoTestAccount.pubkey,
      appDataHash: new BN(verifierProgramTwoProgramId.toBuffer()),
      includeAppData: false,
      verifierAddress: new PublicKey(
        lightProvider.lookUpTables.verifierProgramLookupTable[1],
      ),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    const encBytesVecs: Uint8Array[] = [];

    for (let i = 0; i < 256 * 1000; i++) {
      encBytesVecs.push(randomBytes(120 + UTXO_PREFIX_LENGTH));
    }

    let collisionCounter = 0;

    console.time("getTransactionMerkleTreePdaTime");
    const transactionMerkleTreePda =
      MerkleTreeConfig.getTransactionMerkleTreePda();
    console.timeEnd("getTransactionMerkleTreePdaTime");

    console.time("256kPrefixHashCollisionTestTime");
    for (let i = 0; i < 256 * 1000; i++) {
      const resultUtxo = await Utxo.decrypt({
        poseidon,
        encBytes: encBytesVecs[i],
        account: utxoTestAccount,
        aes: true,
        index: 0,
        merkleTreePdaPublicKey: transactionMerkleTreePda,
        commitment: new BN(testUtxo.getCommitment(poseidon)).toBuffer("le", 32),
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
      });
      if (
        resultUtxo.error &&
        resultUtxo.error.code === UtxoErrorCode.PREFIX_COLLISION
      )
        collisionCounter++;
    }
    console.timeEnd("256kPrefixHashCollisionTestTime");
    console.log(
      "collisions detected (over 256k decryption attempts): ",
      collisionCounter,
    );
  });
});

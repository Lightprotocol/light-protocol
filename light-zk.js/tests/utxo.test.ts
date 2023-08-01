import { assert, expect } from "chai";
import {
  SystemProgram,
  Keypair as SolanaKeypair,
  PublicKey,
} from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { it } from "mocha";
const circomlibjs = require("circomlibjs");
const { buildPoseidonOpt } = circomlibjs;
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);
import { IDL as TEST_PSP_IDL } from "./testData/tmp_test_psp";

import {
  FEE_ASSET,
  hashAndTruncateToCircuit,
  Provider as LightProvider,
  MINT,
  Relayer,
  UtxoError,
  UtxoErrorCode,
  Utxo,
  Account,
  verifierProgramTwoProgramId,
  MerkleTreeConfig,
  createAccountObject,
} from "../src";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

describe("Utxo Functional", () => {
  let seed32 = bs58.encode(new Uint8Array(32).fill(1));
  let depositAmount = 20_000;
  let depositFeeAmount = 10_000;

  let mockPubkey = SolanaKeypair.generate().publicKey;
  let relayerMockPubKey = SolanaKeypair.generate().publicKey;
  let poseidon: any,
    lightProvider: LightProvider,
    deposit_utxo1,
    relayer,
    keypair;
  before(async () => {
    poseidon = await buildPoseidonOpt();
    // TODO: make fee mandatory
    relayer = new Relayer(relayerMockPubKey, mockPubkey, new BN(5000));
    keypair = new Account({ poseidon: poseidon, seed: seed32 });
    lightProvider = await LightProvider.loadMock();
    deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(depositFeeAmount), new BN(depositAmount)],
      account: keypair,
      index: 1,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
  });

  it("rnd utxo functional loop 100", async () => {
    for (let i = 0; i < 100; i++) {
      // try basic tests for rnd empty utxo
      const utxo4Account = new Account({ poseidon });
      const utxo4 = new Utxo({
        poseidon,
        amounts: [new BN(123)],
        account: utxo4Account,
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
      const encBytes4 = await utxo4.encrypt(
        poseidon,
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      );
      const encBytes41 = await utxo4.encrypt(
        poseidon,
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      );
      assert.equal(encBytes4.toString(), encBytes41.toString());
      const utxo41 = await Utxo.decrypt({
        poseidon,
        encBytes: encBytes4,
        account: utxo4Account,
        index: 0,
        merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
        commitment: new anchor.BN(utxo4.getCommitment(poseidon)).toBuffer(
          "le",
          32,
        ),
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
      });

      if (utxo41) {
        Utxo.equal(poseidon, utxo4, utxo41);
      } else {
        throw new Error("decrypt failed");
      }
    }
  });

  it("toString", async () => {
    const amountFee = "1";
    const amountToken = "2";
    const assetPubkey = MINT;
    const seed32 = new Uint8Array(32).fill(1).toString();
    let inputs = {
      keypair: new Account({ poseidon, seed: seed32 }),
      amountFee,
      amountToken,
      assetPubkey,
      assets: [SystemProgram.programId, assetPubkey],
      amounts: [new BN(amountFee), new BN(amountToken)],
      blinding: new BN(new Uint8Array(31).fill(2)),
      index: 1,
    };

    let utxo0 = new Utxo({
      poseidon,
      assets: inputs.assets,
      amounts: inputs.amounts,
      account: inputs.keypair,
      blinding: inputs.blinding,
      index: inputs.index,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    let string = await utxo0.toString();
    let utxo1 = Utxo.fromString(
      string,
      poseidon,
      lightProvider.lookUpTables.assetLookupTable,
      lightProvider.lookUpTables.verifierProgramLookupTable,
    );
    // cannot compute nullifier in utxo1 because no privkey is serialized with toString()
    Utxo.equal(poseidon, utxo0, utxo1, true);
  });

  it("toString", async () => {
    const amountFee = "1";
    const amountToken = "2";
    const assetPubkey = MINT;
    const seed32 = new Uint8Array(32).fill(1).toString();
    let inputs = {
      keypair: new Account({ poseidon, seed: seed32 }),
      amountFee,
      amountToken,
      assetPubkey,
      assets: [SystemProgram.programId, assetPubkey],
      amounts: [new BN(amountFee), new BN(amountToken)],
      blinding: new BN(new Uint8Array(31).fill(2)),
      index: 1,
    };

    let utxo0 = new Utxo({
      poseidon,
      assets: inputs.assets,
      amounts: inputs.amounts,
      account: inputs.keypair,
      blinding: inputs.blinding,
      index: inputs.index,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    let string = await utxo0.toString();
    let utxo1 = Utxo.fromString(
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
    let inputs = {
      keypair: new Account({ poseidon, seed: seed32 }),
      amountFee,
      amountToken,
      assetPubkey,
      assets: [SystemProgram.programId, assetPubkey],
      amounts: [new BN(amountFee), new BN(amountToken)],
      blinding: new BN(new Uint8Array(31).fill(2)),
      index: 1,
    };

    let utxo0 = new Utxo({
      poseidon,
      assets: inputs.assets,
      amounts: inputs.amounts,
      account: inputs.keypair,
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
      utxo0.getNullifier(poseidon)?.toString(),
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
    const encBytes = await utxo1.encrypt(
      poseidon,
      MerkleTreeConfig.getTransactionMerkleTreePda(),
    );

    // decrypt
    const utxo3 = await Utxo.decrypt({
      poseidon,
      encBytes,
      account: inputs.keypair,
      index: inputs.index,
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      commitment: new anchor.BN(utxo1.getCommitment(poseidon)).toBuffer(
        "le",
        32,
      ),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    if (utxo3) {
      Utxo.equal(poseidon, utxo0, utxo3);
    } else {
      throw new Error("decrypt failed");
    }
    let pubKey = inputs.keypair.getPublicKey();
    // encrypting with nacl because this utxo's account does not have an aes secret key since it is instantiated from a public key
    const receivingUtxo = new Utxo({
      poseidon,
      assets: inputs.assets,
      amounts: inputs.amounts,
      account: Account.fromPubkey(pubKey, poseidon),
      blinding: inputs.blinding,
      index: inputs.index,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    // encrypt
    const encBytesNacl = await receivingUtxo.encrypt(
      poseidon,
      MerkleTreeConfig.getTransactionMerkleTreePda(),
    );

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
    if (receivingUtxo1) {
      Utxo.equal(poseidon, receivingUtxo, receivingUtxo1, true);
    } else {
      throw new Error("decrypt failed");
    }
  });

  it("Program utxo to/from bytes ", async () => {
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
      account,
      amounts: [new BN(1_000_000)],
      appData: { releaseSlot: new BN(1) },
      appDataIdl: TEST_PSP_IDL,
      verifierAddress: verifierProgramId,
      index: 0,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    let bytes = await outputUtxo.toBytes();

    let utxo1 = Utxo.fromBytes({
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
    let data = createAccountObject(
      {
        releaseSlot: 1,
        rndOtherStuff: { s: 2342 },
        o: [2, 2, new BN(2)],
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
        { rndOtherStuff: { s: 2342 }, o: [2, 2, new BN(2)] },
        TEST_PSP_IDL.accounts,
        "utxoAppData",
      );
    }).to.throw(Error);
  });
});

describe("Utxo Errors", () => {
  let seed32 = bs58.encode(new Uint8Array(32).fill(1));

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
    let publicKey = keypair.getPublicKey();
    let account = Account.fromPubkey(publicKey, poseidon);
    let pubkeyUtxo = new Utxo({
      poseidon,
      amounts: [new BN(1)],
      account,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    expect(() => {
      pubkeyUtxo.getNullifier(poseidon);
    })
      .throw(UtxoError)
      .include({
        code: UtxoErrorCode.INDEX_NOT_PROVIDED,
        functionName: "getNullifier",
      });
  });

  it("get nullifier without private key", async () => {
    let publicKey = keypair.getPublicKey();

    let account = Account.fromPubkey(publicKey, poseidon);
    let pubkeyUtxo = new Utxo({
      poseidon,
      amounts: [new BN(1)],
      account,
      index: 1,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    expect(() => {
      pubkeyUtxo.getNullifier(poseidon);
    })
      .throw(UtxoError)
      .include({
        code: UtxoErrorCode.ACCOUNT_HAS_NO_PRIVKEY,
        functionName: "getNullifier",
      });
  });

  it("INVALID_ASSET_OR_AMOUNTS_LENGTH", () => {
    expect(() => {
      new Utxo({
        poseidon,
        assets: [inputs.assets[1]],
        amounts: inputs.amounts,
        account: inputs.keypair,
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
        amounts: [new BN(1), new BN(1), new BN(1)],
        account: inputs.keypair,
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
        account: inputs.keypair,
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
        account: inputs.keypair,
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
        account: inputs.keypair,
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
});

import { assert, expect } from "chai";
import { SystemProgram, Keypair as SolanaKeypair, PublicKey } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { it } from "mocha";
import { buildPoseidonOpt } from "circomlibjs";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");

// Load chai-as-promised support
chai.use(chaiAsPromised);
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
  newNonce,
  MERKLE_TREE_KEY,
  verifierProgramTwoProgramId,
} from "../src";
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

describe("Utxo Functional", () => {
  let seed32 = new Uint8Array(32).fill(1).toString();
  let depositAmount = 20_000;
  let depositFeeAmount = 10_000;

  let mockPubkey = SolanaKeypair.generate().publicKey;
  let mockPubkey1 = SolanaKeypair.generate().publicKey;
  let mockPubkey2 = SolanaKeypair.generate().publicKey;
  let mockPubkey3 = SolanaKeypair.generate().publicKey;
  let poseidon, lightProvider, deposit_utxo1, outputUtxo, relayer, keypair;
  before(async () => {
    poseidon = await buildPoseidonOpt();
    // TODO: make fee mandatory
    relayer = new Relayer(
      mockPubkey3,
      mockPubkey,
      mockPubkey,
      new anchor.BN(5000),
    );
    keypair = new Account({ poseidon: poseidon, seed: seed32 });
    lightProvider = await LightProvider.loadMock();
    deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
      index: 1,
    });
  });

  it("rnd utxo functional", async () => {
    // try basic tests for rnd empty utxo
    const utxo4Account = new Account({poseidon});
    const utxo4 = new Utxo({ poseidon, amounts: [new anchor.BN(123)], account:  utxo4Account, appDataHash: new anchor.BN(verifierProgramTwoProgramId.toBuffer()),includeAppData: false, verifierAddress: new PublicKey("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS") });

    // toBytes
    const bytes4 = await utxo4.toBytes();

    // fromBytes
    const utxo40 = Utxo.fromBytes({
      poseidon,
      bytes: bytes4,
      index: 0,
    });
    Utxo.equal(poseidon,utxo4, utxo40);

    // toBytes
    const bytes4Compressed = await utxo4.toBytes(true);

    // fromBytes
    const utxo40Compressed = Utxo.fromBytes({
      poseidon,
      account: utxo4Account,
      bytes: bytes4Compressed,
      index: 0,
    });    
    Utxo.equal(poseidon,utxo4, utxo40Compressed);


    // encrypt
    const encBytes4 = await utxo4.encrypt(poseidon, MERKLE_TREE_KEY, 0);
    const encBytes41 = await utxo4.encrypt(poseidon, MERKLE_TREE_KEY, 0);
    assert.equal(encBytes4.toString(), encBytes41.toString());
    const utxo41 = await Utxo.decrypt({
      poseidon,
      encBytes: encBytes4,
      account: utxo4Account,
      index: 0,
      merkleTreePdaPublicKey: MERKLE_TREE_KEY,
      commitment: new anchor.BN(utxo4.getCommitment(poseidon)).toBuffer("le", 32),
      transactionIndex: 0
    });

    if (utxo41) {
      Utxo.equal(poseidon,utxo4, utxo41);
    } else {
      throw new Error("decrypt failed");
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
      amounts: [new anchor.BN(amountFee), new anchor.BN(amountToken)],
      blinding: new anchor.BN(new Uint8Array(31).fill(2)),
      index: 1,
    };

    let utxo0 = new Utxo({
      poseidon,
      assets: inputs.assets,
      amounts: inputs.amounts,
      account: inputs.keypair,
      blinding: inputs.blinding,
      index: inputs.index,
    });
    let string = await utxo0.toString();
    let utxo1 = Utxo.fromString(string, poseidon);
    // cannot comput nullifier in utxo1 because no privkey is serialized with toString()
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
      amounts: [new anchor.BN(amountFee), new anchor.BN(amountToken)],
      blinding: new anchor.BN(new Uint8Array(31).fill(2)),
      index: 1,
    };

    let utxo0 = new Utxo({
      poseidon,
      assets: inputs.assets,
      amounts: inputs.amounts,
      account: inputs.keypair,
      blinding: inputs.blinding,
      index: inputs.index,
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
      "8989324955018347745620195382288710751873914589499358508918782406019233094196",
    );

    assert.equal(
      utxo0.getNullifier(poseidon)?.toString(),
      "16754375772623288827522514885252653352689437303609900913797444969754165213445",
    );

    // toBytes
    const bytes = await utxo0.toBytes();
    // fromBytes
    const utxo1 = Utxo.fromBytes({
      poseidon,
      account: inputs.keypair,
      bytes,
      index: inputs.index,
    });
    Utxo.equal(poseidon,utxo0, utxo1);

    // encrypt
    const encBytes = await utxo1.encrypt(poseidon, MERKLE_TREE_KEY, 0);

    // decrypt
    const utxo3 = await Utxo.decrypt({
      poseidon,
      encBytes,
      account: inputs.keypair,
      index: inputs.index,
      merkleTreePdaPublicKey: MERKLE_TREE_KEY,
      transactionIndex: 0,
      commitment: new anchor.BN(utxo1.getCommitment(poseidon)).toBuffer("le", 32),
    });
    if (utxo3) {
      Utxo.equal(poseidon,utxo0, utxo3);
    } else {
      throw new Error("decrypt failed");
    }
    // encrypting with nacl because this utxo's account does not have an aes secret key since it is instantiated from a public key
    const receivingUtxo = new Utxo({
      poseidon,
      assets: inputs.assets,
      amounts: inputs.amounts,
      account: Account.fromPubkey(inputs.keypair.pubkey.toBuffer(), inputs.keypair.encryptionKeypair.publicKey, poseidon),
      blinding: inputs.blinding,
      index: inputs.index,
    });
    // encrypt
    const encBytesNacl = await receivingUtxo.encrypt(poseidon, MERKLE_TREE_KEY, 0);

    // decrypt
    const receivingUtxo1 = await Utxo.decrypt({
      poseidon,
      encBytes: encBytesNacl,
      account: inputs.keypair,
      index: inputs.index,
      merkleTreePdaPublicKey: MERKLE_TREE_KEY,
      transactionIndex: 0,
      aes: false,
      commitment: new anchor.BN(receivingUtxo.getCommitment(poseidon)).toBuffer("le", 32),
    });
    if (receivingUtxo1) {
      Utxo.equal(poseidon,receivingUtxo, receivingUtxo1, true);
    } else {
      throw new Error("decrypt failed");
    }
  });
});

describe("Utxo Errors", () => {
  let seed32 = new Uint8Array(32).fill(1).toString();

  let poseidon, inputs, keypair;

  const amountFee = "1";
  const amountToken = "2";
  const assetPubkey = MINT;

  before(async () => {
    poseidon = await buildPoseidonOpt();
    keypair = new Account({ poseidon: poseidon, seed: seed32 });
    inputs = {
      keypair: new Account({ poseidon, seed: seed32 }),
      amountFee,
      amountToken,
      assetPubkey,
      assets: [SystemProgram.programId, assetPubkey],
      amounts: [new anchor.BN(amountFee), new anchor.BN(amountToken)],
      blinding: new anchor.BN(new Uint8Array(31).fill(2)),
    };
  });

  it("get nullifier without index", async () => {
    let account = Account.fromPubkey(
      keypair.pubKey,
      keypair.encryptionKeypair.publicKey,
      poseidon,
    );
    let pubkeyUtxo = new Utxo({
      poseidon,
      amounts: [new anchor.BN(1)],
      account,
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
    let account = Account.fromPubkey(
      keypair.pubKey,
      keypair.encryptionKeypair.publicKey,
      poseidon,
    );
    let pubkeyUtxo = new Utxo({
      poseidon,
      amounts: [new anchor.BN(1)],
      account,
      index: 1,
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
      });
    })
      .to.throw(UtxoError)
      .to.include({
        code: UtxoErrorCode.INVALID_ASSET_OR_AMOUNTS_LENGTH,
        codeMessage: "Length missmatch assets: 1 != amounts: 2",
      });
  });

  it("EXCEEDED_MAX_ASSETS", () => {
    expect(() => {
      new Utxo({
        poseidon,
        assets: [MINT, MINT, MINT],
        amounts: [new anchor.BN(1), new anchor.BN(1), new anchor.BN(1)],
        account: inputs.keypair,
        blinding: inputs.blinding,
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
        amounts: [inputs.amounts[0], new anchor.BN(-1)],
        account: inputs.keypair,
        blinding: inputs.blinding,
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
      });
    })
      .to.throw(UtxoError)
      .to.include({
        code: UtxoErrorCode.APP_DATA_IDL_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("ASSET_NOT_FOUND", async () => {
    await chai.assert.isRejected(
      new Utxo({
        poseidon,
        assets: [SystemProgram.programId, SolanaKeypair.generate().publicKey],
        amounts: inputs.amounts,
        account: inputs.keypair,
        blinding: inputs.blinding,
      }).toBytes()      
    ,
    UtxoErrorCode.ASSET_NOT_FOUND
    )
  });
});

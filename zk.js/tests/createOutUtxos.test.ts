import { assert, expect } from "chai";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
import { BN } from "@coral-xyz/anchor";
// Load chai-as-promised support
chai.use(chaiAsPromised);

import {
  SystemProgram,
  Keypair as SolanaKeypair,
  PublicKey,
} from "@solana/web3.js";
import { it } from "mocha";

import {
  Action,
  TransactionParametersErrorCode,
  createOutUtxos,
  TOKEN_REGISTRY,
  CreateUtxoError,
  CreateUtxoErrorCode,
  Account,
  MINT,
  validateUtxoAmounts,
  Recipient,
  createRecipientUtxos,
  Provider,
  TokenData,
  RPC_FEE,
  BN_0,
  BN_1,
  BN_2,
  createOutUtxo,
  Utxo,
  OutUtxo,
  createUtxo,
} from "../src";
import { LightWasm, WasmFactory } from "@lightprotocol/account.rs";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
const numberMaxOutUtxos = 2;

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

const seed32 = bs58.encode(new Uint8Array(32).fill(1));

describe("Test createOutUtxos Functional", () => {
  let lightWasm: LightWasm, k0: Account;

  let splAmount: BN,
    solAmount: BN,
    token,
    tokenCtx: TokenData,
    utxo1: Utxo,
    rpcFee: BN,
    utxoSol: Utxo,
    recipientAccount: Account,
    lightProvider: Provider;
  before(async () => {
    lightProvider = await Provider.loadMock();
    lightWasm = await WasmFactory.getInstance();
    k0 = Account.createFromSeed(lightWasm, seed32);
    splAmount = new BN(3);
    solAmount = new BN(1e6);
    token = "USDC";
    const tmpTokenCtx = TOKEN_REGISTRY.get(token);
    if (!tmpTokenCtx) throw new Error("Token not supported!");
    tokenCtx = tmpTokenCtx as TokenData;
    splAmount = splAmount.mul(new BN(tokenCtx.decimals));
    utxo1 = createUtxo(
      lightWasm,
      k0,
      {
        assets: [SystemProgram.programId, tokenCtx.mint],
        amounts: [new BN(1e8), new BN(5 * tokenCtx.decimals.toNumber())],
        merkleProof: ["1"],
        utxoHash: "0",
        blinding: "1",
        merkleTreeLeafIndex: 0,
      },
      false,
    );
    utxoSol = createUtxo(
      lightWasm,
      k0,
      {
        assets: [SystemProgram.programId],
        amounts: [new BN(1e6)],
        merkleProof: ["1"],
        utxoHash: "0",
        blinding: "2",
        merkleTreeLeafIndex: 0,
      },
      false,
    );
    rpcFee = RPC_FEE;

    const recipientAccountRoot = Account.createFromSeed(
      lightWasm,
      bs58.encode(new Uint8Array(32).fill(3)),
    );

    recipientAccount = Account.fromPubkey(
      recipientAccountRoot.getPublicKey(),
      lightWasm,
    );
  });

  it("compress sol", async () => {
    const outUtxos = createOutUtxos({
      publicMint: tokenCtx.mint,
      publicAmountSpl: BN_0,
      publicAmountSol: solAmount,
      lightWasm,
      changeUtxoAccount: k0,
      action: Action.COMPRESS,
      numberMaxOutUtxos,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    assert.equal(
      outUtxos[0].amounts[0].toString(),
      solAmount.toString(),
      `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`,
    );
    assert.equal(
      outUtxos[0].amounts[1].toString(),
      "0",
      `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${
        utxo1.amounts[1].toNumber() -
        splAmount.toNumber() * tokenCtx.decimals.toNumber()
      }`,
    );
  });

  it("compress spl", async () => {
    const outUtxos = createOutUtxos({
      publicMint: tokenCtx.mint,
      publicAmountSpl: new BN(10),
      publicAmountSol: BN_0,
      lightWasm,
      changeUtxoAccount: k0,
      action: Action.COMPRESS,
      numberMaxOutUtxos,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    assert.equal(
      outUtxos[0].amounts[0].toString(),
      "0",
      `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`,
    );
    assert.equal(
      outUtxos[0].amounts[1].toString(),
      "10",
      `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${
        utxo1.amounts[1].toNumber() -
        splAmount.toNumber() * tokenCtx.decimals.toNumber()
      }`,
    );
  });

  it("compress sol with input utxo", async () => {
    const outUtxos = createOutUtxos({
      inUtxos: [utxo1],
      publicMint: tokenCtx.mint,
      publicAmountSpl: BN_0,
      publicAmountSol: solAmount,
      lightWasm,
      changeUtxoAccount: k0,
      action: Action.COMPRESS,
      numberMaxOutUtxos,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    assert.equal(
      outUtxos[0].amounts[0].toString(),
      utxo1.amounts[0].add(solAmount).toString(),
      `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`,
    );
    assert.equal(
      outUtxos[0].amounts[1].toString(),
      utxo1.amounts[1].toString(),
      `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${
        utxo1.amounts[1].toNumber() -
        splAmount.toNumber() * tokenCtx.decimals.toNumber()
      }`,
    );
  });

  it("compress sol & spl with input utxo", async () => {
    const outUtxos = createOutUtxos({
      inUtxos: [utxo1],
      publicMint: tokenCtx.mint,
      publicAmountSpl: new BN(10),
      publicAmountSol: solAmount,
      lightWasm,
      changeUtxoAccount: k0,
      action: Action.COMPRESS,
      numberMaxOutUtxos,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    assert.equal(
      outUtxos[0].amounts[0].toString(),
      utxo1.amounts[0].add(solAmount).toString(),
      `${outUtxos[0].amounts[0].add(solAmount)} fee != ${utxo1.amounts[0]}`,
    );
    assert.equal(
      outUtxos[0].amounts[1].toString(),
      utxo1.amounts[1].add(new BN("10")).toString(),
      `${utxo1.amounts[1].add(new BN("10")).toString()}  spl !=  ${
        utxo1.amounts[1].toNumber() -
        splAmount.toNumber() * tokenCtx.decimals.toNumber()
      }`,
    );
  });

  it("compress sol & spl", async () => {
    const outUtxos = createOutUtxos({
      publicMint: tokenCtx.mint,
      publicAmountSpl: new BN(10),
      publicAmountSol: solAmount,
      lightWasm,
      changeUtxoAccount: k0,
      action: Action.COMPRESS,
      numberMaxOutUtxos,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    assert.equal(
      outUtxos[0].amounts[0].toString(),
      solAmount.toString(),
      `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`,
    );
    assert.equal(
      outUtxos[0].amounts[1].toString(),
      "10",
      `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${
        utxo1.amounts[1].toNumber() -
        splAmount.toNumber() * tokenCtx.decimals.toNumber()
      }`,
    );
  });

  it("decompress SPL - no rpc fee", async () => {
    const outUtxos = createOutUtxos({
      inUtxos: [utxo1],
      publicMint: tokenCtx.mint,
      publicAmountSpl: splAmount,
      publicAmountSol: BN_0,
      lightWasm,
      rpcFee: BN_0,
      changeUtxoAccount: k0,
      action: Action.DECOMPRESS,
      numberMaxOutUtxos,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    assert.equal(
      outUtxos[0].amounts[0].toString(),
      utxo1.amounts[0].toString(),
      `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`,
    );
    assert.equal(
      outUtxos[0].amounts[1].toString(),
      utxo1.amounts[1].sub(splAmount).toString(),
      `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${
        utxo1.amounts[1].toNumber() -
        splAmount.toNumber() * tokenCtx.decimals.toNumber()
      }`,
    );
  });

  it("decompress SPL - with rpc fee", async () => {
    const outUtxos = createOutUtxos({
      inUtxos: [utxo1],
      publicMint: tokenCtx.mint,
      publicAmountSpl: splAmount,
      publicAmountSol: BN_0,
      lightWasm,
      rpcFee,
      changeUtxoAccount: k0,
      action: Action.DECOMPRESS,
      numberMaxOutUtxos,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    assert.equal(
      outUtxos[0].amounts[0].toString(),
      utxo1.amounts[0].sub(rpcFee).toString(),
      `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`,
    );
    assert.equal(
      outUtxos[0].amounts[1].toString(),
      utxo1.amounts[1].sub(splAmount).toString(),
      `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${
        utxo1.amounts[1].toNumber() -
        splAmount.toNumber() * tokenCtx.decimals.toNumber()
      }`,
    );
  });

  it("decompress sol - no rpc fee", async () => {
    const outUtxos = createOutUtxos({
      inUtxos: [utxo1],
      publicMint: tokenCtx.mint,
      publicAmountSpl: BN_0,
      publicAmountSol: solAmount,
      lightWasm,
      rpcFee: BN_0,
      changeUtxoAccount: k0,
      action: Action.DECOMPRESS,
      numberMaxOutUtxos,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    assert.equal(
      outUtxos[0].amounts[0].toString(),
      utxo1.amounts[0].sub(solAmount).toString(),
      `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`,
    );
    assert.equal(
      outUtxos[0].amounts[1].toString(),
      utxo1.amounts[1].toString(),
      `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${
        utxo1.amounts[1].toNumber() -
        splAmount.toNumber() * tokenCtx.decimals.toNumber()
      }`,
    );
  });

  it("decompress sol - with rpc fee", async () => {
    const outUtxos = createOutUtxos({
      inUtxos: [utxo1],
      publicMint: tokenCtx.mint,
      publicAmountSpl: BN_0,
      publicAmountSol: solAmount,
      lightWasm,
      rpcFee,
      changeUtxoAccount: k0,
      action: Action.DECOMPRESS,
      numberMaxOutUtxos,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    assert.equal(
      outUtxos[0].amounts[0].toString(),
      utxo1.amounts[0].sub(rpcFee).sub(solAmount).toString(),
      `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`,
    );
    assert.equal(
      outUtxos[0].amounts[1].toString(),
      utxo1.amounts[1].toString(),
      `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${
        utxo1.amounts[1].toNumber() -
        splAmount.toNumber() * tokenCtx.decimals.toNumber()
      }`,
    );
  });

  it("decompress spl & sol - no rpc fee", async () => {
    const outUtxos = createOutUtxos({
      inUtxos: [utxo1],
      publicMint: tokenCtx.mint,
      publicAmountSpl: splAmount,
      publicAmountSol: solAmount,
      lightWasm,
      rpcFee: BN_0,
      changeUtxoAccount: k0,
      action: Action.DECOMPRESS,
      numberMaxOutUtxos,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    assert.equal(
      outUtxos[0].amounts[0].toString(),
      utxo1.amounts[0].sub(solAmount).toString(),
      `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`,
    );
    assert.equal(
      outUtxos[0].amounts[1].toString(),
      utxo1.amounts[1].sub(splAmount).toString(),
      `${outUtxos[0].amounts[1].sub(splAmount).toNumber()}  spl !=  ${
        utxo1.amounts[1].toNumber() -
        splAmount.toNumber() * tokenCtx.decimals.toNumber()
      }`,
    );
  });

  it("decompress spl & sol - with rpc fee", async () => {
    const outUtxos = createOutUtxos({
      inUtxos: [utxo1],
      publicMint: tokenCtx.mint,
      publicAmountSpl: splAmount,
      publicAmountSol: solAmount,
      lightWasm,
      rpcFee,
      changeUtxoAccount: k0,
      action: Action.DECOMPRESS,
      numberMaxOutUtxos,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    assert.equal(
      outUtxos[0].amounts[0].toString(),
      utxo1.amounts[0].sub(rpcFee).sub(solAmount).toString(),
      `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`,
    );
    assert.equal(
      outUtxos[0].amounts[1].toString(),
      utxo1.amounts[1].sub(splAmount).toString(),
      `${outUtxos[0].amounts[1].sub(splAmount).toNumber()}  spl !=  ${
        utxo1.amounts[1].toNumber() -
        splAmount.toNumber() * tokenCtx.decimals.toNumber()
      }`,
    );
  });

  it("decompress in:1SOL + 1SPL should merge 2-1", async () => {
    const outUtxos = createOutUtxos({
      publicMint: tokenCtx.mint,
      publicAmountSpl: splAmount,
      inUtxos: [utxo1, utxoSol],
      publicAmountSol: BN_0,
      lightWasm,
      changeUtxoAccount: k0,
      action: Action.DECOMPRESS,
      numberMaxOutUtxos,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    assert.equal(
      outUtxos[0].amounts[0].toNumber(),
      utxo1.amounts[0].toNumber() + utxoSol.amounts[0].toNumber(),
      `${outUtxos[0].amounts[0]} fee != ${
        utxo1.amounts[0].toNumber() + utxoSol.amounts[0].toNumber()
      }`,
    );
    assert.equal(
      outUtxos[0].amounts[1].toString(),
      utxo1.amounts[1].sub(splAmount).toString(),
      `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${
        utxo1.amounts[1].toNumber() -
        splAmount.toNumber() * tokenCtx.decimals.toNumber()
      }`,
    );
  });

  it("decompress in:1SPL + 1SPL should merge 2-1", async () => {
    const outUtxos = createOutUtxos({
      publicMint: tokenCtx.mint,
      publicAmountSpl: splAmount,
      inUtxos: [utxo1, utxo1],
      publicAmountSol: BN_0,
      lightWasm,
      changeUtxoAccount: k0,
      action: Action.DECOMPRESS,
      numberMaxOutUtxos,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    assert.equal(
      outUtxos[0].amounts[0].toNumber(),
      utxo1.amounts[0].mul(BN_2).toNumber(),
      `${outUtxos[0].amounts[0]} fee != ${
        utxo1.amounts[0].toNumber() + utxo1.amounts[0].toNumber()
      }`,
    );
    assert.equal(
      outUtxos[0].amounts[1].toString(),
      utxo1.amounts[1].mul(BN_2).sub(splAmount).toString(),
      `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${
        utxo1.amounts[1].toNumber() - splAmount.toNumber()
      }`,
    );
  });

  it("transfer in:1 SPL ", async () => {
    const recipients = [
      {
        account: recipientAccount,
        mint: utxo1.assets[1],
        solAmount: BN_0,
        splAmount: BN_1,
      },
    ];
    let outUtxos = createRecipientUtxos({
      recipients,
      lightWasm,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    outUtxos = createOutUtxos({
      publicMint: tokenCtx.mint,
      publicAmountSpl: BN_0,
      inUtxos: [utxo1],
      outUtxos,
      rpcFee,
      publicAmountSol: BN_0,
      lightWasm,
      changeUtxoAccount: k0,
      action: Action.TRANSFER,
      numberMaxOutUtxos,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    assert.equal(
      outUtxos[1].amounts[0].toNumber(),
      utxo1.amounts[0].toNumber() -
        rpcFee.toNumber() -
        outUtxos[0].amounts[0].toNumber(),
      `${outUtxos[1].amounts[0]} fee != ${
        utxo1.amounts[0].toNumber() -
        rpcFee.toNumber() -
        outUtxos[0].amounts[0].toNumber()
      }`,
    );

    assert.equal(
      outUtxos[1].amounts[1].toNumber(),
      utxo1.amounts[1].toNumber() - 1,
      `${outUtxos[1].amounts[1].toNumber()}  spl !=  ${
        utxo1.amounts[1].toNumber() - splAmount.toNumber()
      }`,
    );
  });
});

// ... existing imports and code ...

describe("createRecipientUtxos", () => {
  let lightProvider: Provider;
  it("should create output UTXOs for each recipient", async () => {
    lightProvider = await Provider.loadMock();
    const lightWasm = await WasmFactory.getInstance();

    const mint = MINT;
    const account1 = Account.createFromSeed(lightWasm, seed32);
    const account2 = Account.createFromSeed(
      lightWasm,
      bs58.encode(new Uint8Array(32).fill(4)),
    );

    const recipients: Recipient[] = [
      {
        account: account1,
        solAmount: new BN(5),
        splAmount: new BN(10),
        mint,
      },
      {
        account: account2,
        solAmount: new BN(3),
        splAmount: new BN(7),
        mint,
      },
    ];

    const outputUtxos = createRecipientUtxos({
      recipients,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      lightWasm,
    });

    expect(outputUtxos.length).to.equal(recipients.length);
    expect(outputUtxos[0].publicKey.toString()).to.equal(
      account1.keypair.publicKey.toString(),
    );
    expect(outputUtxos[0].amounts[0].toString()).to.equal("5");
    expect(outputUtxos[0].amounts[1].toString()).to.equal("10");
    expect(outputUtxos[0].assets[0].equals(SystemProgram.programId)).to.be.true;
    expect(outputUtxos[0].assets[1].equals(mint)).to.be.true;

    expect(outputUtxos[1].publicKey.toString()).to.equal(
      account2.keypair.publicKey.toString(),
    );
    expect(outputUtxos[1].amounts[0].toString()).to.equal("3");
    expect(outputUtxos[1].amounts[1].toString()).to.equal("7");
    expect(outputUtxos[1].assets[0].equals(SystemProgram.programId)).to.be.true;
    expect(outputUtxos[1].assets[1].equals(mint)).to.be.true;
  });
});

describe("validateUtxoAmounts", () => {
  let lightWasm: LightWasm,
    assetPubkey: PublicKey,
    inUtxos: [Utxo, Utxo],
    lightProvider: Provider;
  before(async () => {
    lightProvider = await Provider.loadMock();
    lightWasm = await WasmFactory.getInstance();
    assetPubkey = new PublicKey(0);
    inUtxos = [
      createUtxoHelper(lightWasm, [new BN(5)], [assetPubkey]),
      createUtxoHelper(lightWasm, [new BN(3)], [assetPubkey]),
    ];
  });
  function createUtxoHelper(
    lightWasm: LightWasm,
    amounts: BN[],
    assets: PublicKey[],
  ): Utxo {
    return createUtxo(
      lightWasm,
      Account.createFromSeed(lightWasm, seed32),
      {
        amounts,
        assets,
        blinding: "0",
        utxoHash: "0",
        merkleProof: ["1"],
        merkleTreeLeafIndex: 0,
      },
      false,
    );
  }
  // Helper function to create a UTXO with specific amounts and assets
  function createOutUtxoHelper(
    lightWasm: LightWasm,
    amounts: BN[],
    assets: PublicKey[],
  ): OutUtxo {
    return createOutUtxo({
      lightWasm,
      amounts,
      assets,
      blinding: BN_0,
      publicKey: Account.createFromSeed(lightWasm, seed32).keypair.publicKey,
    });
  }

  it("should not throw an error if input UTXOs sum is equal to output UTXOs sum", () => {
    const outUtxos = [
      createOutUtxoHelper(lightWasm, [new BN(8)], [assetPubkey]),
    ];

    expect(() =>
      validateUtxoAmounts({ assetPubkeys: [assetPubkey], inUtxos, outUtxos }),
    ).not.to.throw();
  });

  it("should not throw an error if input UTXOs sum is greater than output UTXOs sum", () => {
    const outUtxos = [
      createOutUtxoHelper(lightWasm, [new BN(7)], [assetPubkey]),
    ];

    expect(() =>
      validateUtxoAmounts({ assetPubkeys: [assetPubkey], inUtxos, outUtxos }),
    ).not.to.throw();
  });

  it("should throw an error if input UTXOs sum is less than output UTXOs sum", () => {
    const outUtxos = [
      createOutUtxoHelper(lightWasm, [new BN(9)], [assetPubkey]),
    ];

    expect(() =>
      validateUtxoAmounts({ assetPubkeys: [assetPubkey], inUtxos, outUtxos }),
    ).to.throw(CreateUtxoError);
  });
});

describe("Test createOutUtxos Errors", () => {
  let lightWasm: LightWasm, k0: Account;

  let splAmount: BN,
    token,
    tokenCtx: TokenData,
    utxo1: Utxo,
    utxoSol: Utxo,
    lightProvider: Provider;
  before(async () => {
    lightProvider = await Provider.loadMock();
    lightWasm = await WasmFactory.getInstance();
    k0 = Account.createFromSeed(lightWasm, seed32);
    splAmount = new BN(3);
    token = "USDC";
    const tmpTokenCtx = TOKEN_REGISTRY.get(token);
    if (!tmpTokenCtx) throw new Error("Token not supported!");
    tokenCtx = tmpTokenCtx as TokenData;
    splAmount = splAmount.mul(new BN(tokenCtx.decimals));
    utxo1 = createUtxo(
      lightWasm,
      k0,
      {
        assets: [SystemProgram.programId, tokenCtx.mint],
        amounts: [new BN(1e8), new BN(5 * tokenCtx.decimals.toNumber())],
        merkleProof: ["1"],
        utxoHash: "0",
        blinding: "1",
        merkleTreeLeafIndex: 0,
      },
      false,
    );
    utxoSol = createUtxo(
      lightWasm,
      k0,
      {
        assets: [SystemProgram.programId],
        amounts: [new BN(1e6)],
        merkleProof: ["1"],
        utxoHash: "0",
        blinding: "2",
        merkleTreeLeafIndex: 0,
      },
      false,
    );

    createOutUtxos({
      publicMint: tokenCtx.mint,
      publicAmountSpl: splAmount,
      inUtxos: [utxo1, utxoSol],
      publicAmountSol: BN_0,
      changeUtxoAccount: k0,
      action: Action.DECOMPRESS,
      lightWasm: lightWasm,
      numberMaxOutUtxos,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
  });

  it("NO_POSEIDON_HASHER_PROVIDED", async () => {
    expect(() => {
      // @ts-ignore
      createOutUtxos({
        publicMint: tokenCtx.mint,
        publicAmountSpl: splAmount,
        inUtxos: [utxo1, utxoSol],
        publicAmountSol: BN_0,
        // poseidon,
        changeUtxoAccount: k0,
        action: Action.DECOMPRESS,
      });
    })
      .to.throw(CreateUtxoError)
      .includes({
        code: TransactionParametersErrorCode.NO_POSEIDON_HASHER_PROVIDED,
        functionName: "createOutUtxos",
      });
  });

  it("INVALID_NUMBER_OF_RECIPIENTS", async () => {
    expect(() => {
      createOutUtxos({
        publicMint: tokenCtx.mint,
        publicAmountSpl: splAmount,
        inUtxos: [utxo1, utxoSol],
        publicAmountSol: BN_0,
        lightWasm: lightWasm,
        changeUtxoAccount: k0,
        action: Action.DECOMPRESS,
        outUtxos: [
          createOutUtxo({
            lightWasm,
            publicKey: k0.keypair.publicKey,
            amounts: [BN_0],
            assets: [SystemProgram.programId],
          }),
          createOutUtxo({
            lightWasm,
            publicKey: k0.keypair.publicKey,
            amounts: [BN_0],
            assets: [SystemProgram.programId],
          }),
        ],
        numberMaxOutUtxos,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
      });
    })
      .to.throw(CreateUtxoError)
      .includes({
        code: CreateUtxoErrorCode.INVALID_NUMBER_OF_RECIPIENTS,
        functionName: "createOutUtxos",
      });
  });

  it("INVALID_RECIPIENT_MINT", async () => {
    const invalidMint = SolanaKeypair.generate().publicKey;
    expect(() => {
      // @ts-ignore
      createOutUtxos({
        publicMint: tokenCtx.mint,
        publicAmountSpl: splAmount,
        inUtxos: [utxo1, utxoSol],
        publicAmountSol: BN_0,
        lightWasm: lightWasm,
        changeUtxoAccount: k0,
        action: Action.DECOMPRESS,
        outUtxos: [
          createOutUtxo({
            lightWasm,
            publicKey: k0.keypair.publicKey,
            amounts: [BN_0, BN_1],
            assets: [SystemProgram.programId, invalidMint],
          }),
        ],
      });
    })
      .to.throw(CreateUtxoError)
      .includes({
        code: CreateUtxoErrorCode.INVALID_RECIPIENT_MINT,
        functionName: "createOutUtxos",
      });
  });

  it("RECIPIENTS_SUM_AMOUNT_MISMATCH", async () => {
    expect(() => {
      // @ts-ignore
      createOutUtxos({
        publicMint: tokenCtx.mint,
        publicAmountSpl: splAmount,
        inUtxos: [utxo1, utxoSol],
        publicAmountSol: BN_0,
        lightWasm: lightWasm,
        changeUtxoAccount: k0,
        action: Action.DECOMPRESS,
        outUtxos: [
          createOutUtxo({
            lightWasm,
            publicKey: k0.keypair.publicKey,
            amounts: [BN_0, new BN(1e12)],
            assets: [SystemProgram.programId, utxo1.assets[1]],
          }),
        ],
      });
    })
      .to.throw(CreateUtxoError)
      .includes({
        code: CreateUtxoErrorCode.RECIPIENTS_SUM_AMOUNT_MISSMATCH,
        functionName: "validateUtxoAmounts",
      });
  });

  it("NO_PUBLIC_AMOUNTS_PROVIDED", async () => {
    expect(() => {
      // @ts-ignore
      createOutUtxos({
        publicMint: tokenCtx.mint,
        // publicAmountSpl: splAmount,
        inUtxos: [utxo1, utxoSol],
        // publicAmountSol: BN_0,
        lightWasm: lightWasm,
        changeUtxoAccount: k0,
        action: Action.DECOMPRESS,
      });
    })
      .to.throw(CreateUtxoError)
      .includes({
        code: CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED,
        functionName: "createOutUtxos",
      });
  });

  it("NO_PUBLIC_MINT_PROVIDED", async () => {
    expect(() => {
      // @ts-ignore
      createOutUtxos({
        // publicMint: tokenCtx.mint,
        publicAmountSpl: splAmount,
        inUtxos: [utxo1, utxoSol],
        // publicAmountSol: BN_0,
        lightWasm: lightWasm,
        changeUtxoAccount: k0,
        action: Action.DECOMPRESS,
        rpcFee: BN_1,
      });
    })
      .to.throw(CreateUtxoError)
      .includes({
        code: CreateUtxoErrorCode.NO_PUBLIC_MINT_PROVIDED,
        functionName: "createOutUtxos",
      });
  });

  it("INVALID_OUTPUT_UTXO_LENGTH", async () => {
    const invalidMint = SolanaKeypair.generate().publicKey;

    const utxoSol0 = createUtxo(
      lightWasm,
      k0,
      {
        assets: [SystemProgram.programId, invalidMint],
        amounts: [new BN(1e6), new BN(1e6)],
        merkleProof: ["3"],
        utxoHash: "0",
        blinding: "1",
        merkleTreeLeafIndex: 0,
      },
      false,
    );

    expect(() => {
      createOutUtxos({
        publicMint: tokenCtx.mint,
        publicAmountSpl: splAmount,
        inUtxos: [utxo1, utxoSol0],
        publicAmountSol: BN_0,
        lightWasm: lightWasm,
        changeUtxoAccount: k0,
        action: Action.DECOMPRESS,
        outUtxos: [
          createOutUtxo({
            lightWasm,
            publicKey: k0.keypair.publicKey,
            amounts: [BN_0, BN_1],
            assets: [SystemProgram.programId, utxo1.assets[1]],
          }),
        ],
        numberMaxOutUtxos,
        assetLookupTable: [
          ...lightProvider.lookUpTables.assetLookupTable,
          ...[invalidMint.toBase58()],
        ],
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
      });
    })
      .to.throw(CreateUtxoError)
      .includes({
        code: CreateUtxoErrorCode.INVALID_OUTPUT_UTXO_LENGTH,
        functionName: "createOutUtxos",
      });
  });
});

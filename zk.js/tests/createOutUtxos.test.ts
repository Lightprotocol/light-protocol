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
  Utxo,
  CreateUtxoError,
  CreateUtxoErrorCode,
  Account,
  MINT,
  validateUtxoAmounts,
  Recipient,
  createRecipientUtxos,
  Provider,
  TokenData,
  RELAYER_FEE,
  BN_0,
  BN_1,
  BN_2,
} from "../src";
import { Hasher, WasmHasher } from "@lightprotocol/account.rs";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
const numberMaxOutUtxos = 2;

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

const seed32 = bs58.encode(new Uint8Array(32).fill(1));

describe("Test createOutUtxos Functional", () => {
  let hasher: Hasher, k0: Account;

  let splAmount: BN,
    solAmount: BN,
    token: string,
    tokenCtx: TokenData,
    utxo1: Utxo,
    relayerFee: BN,
    utxoSol: Utxo,
    recipientAccount: Account,
    lightProvider: Provider;
  before(async () => {
    lightProvider = await Provider.loadMock();
    hasher = await WasmHasher.getInstance();
    k0 = Account.createFromSeed(hasher, seed32);
    splAmount = new BN(3);
    solAmount = new BN(1e6);
    token = "USDC";
    const tmpTokenCtx = TOKEN_REGISTRY.get(token);
    if (!tmpTokenCtx) throw new Error("Token not supported!");
    tokenCtx = tmpTokenCtx as TokenData;
    splAmount = splAmount.mul(new BN(tokenCtx.decimals));
    utxo1 = new Utxo({
      hasher,
      assets: [SystemProgram.programId, tokenCtx.mint],
      amounts: [new BN(1e8), new BN(5 * tokenCtx.decimals.toNumber())],
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      publicKey: k0.keypair.publicKey,
    });
    utxoSol = new Utxo({
      hasher,
      assets: [SystemProgram.programId],
      amounts: [new BN(1e6)],
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      publicKey: k0.keypair.publicKey,
    });
    relayerFee = RELAYER_FEE;

    const recipientAccountRoot = Account.createFromSeed(
      hasher,
      bs58.encode(new Uint8Array(32).fill(3)),
    );

    recipientAccount = Account.fromPubkey(
      recipientAccountRoot.getPublicKey(),
      hasher,
    );
  });

  it("shield sol", async () => {
    const outUtxos = createOutUtxos({
      publicMint: tokenCtx.mint,
      publicAmountSpl: BN_0,
      publicAmountSol: solAmount,
      hasher,
      changeUtxoAccount: k0,
      action: Action.SHIELD,
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

  it("shield spl", async () => {
    const outUtxos = createOutUtxos({
      publicMint: tokenCtx.mint,
      publicAmountSpl: new BN(10),
      publicAmountSol: BN_0,
      hasher,
      changeUtxoAccount: k0,
      action: Action.SHIELD,
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

  it("shield sol with input utxo", async () => {
    const outUtxos = createOutUtxos({
      inUtxos: [utxo1],
      publicMint: tokenCtx.mint,
      publicAmountSpl: BN_0,
      publicAmountSol: solAmount,
      hasher,
      changeUtxoAccount: k0,
      action: Action.SHIELD,
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

  it("shield sol & spl with input utxo", async () => {
    const outUtxos = createOutUtxos({
      inUtxos: [utxo1],
      publicMint: tokenCtx.mint,
      publicAmountSpl: new BN(10),
      publicAmountSol: solAmount,
      hasher,
      changeUtxoAccount: k0,
      action: Action.SHIELD,
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

  it("shield sol & spl", async () => {
    const outUtxos = createOutUtxos({
      publicMint: tokenCtx.mint,
      publicAmountSpl: new BN(10),
      publicAmountSol: solAmount,
      hasher,
      changeUtxoAccount: k0,
      action: Action.SHIELD,
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

  it("unshield SPL - no relayer fee", async () => {
    const outUtxos = createOutUtxos({
      inUtxos: [utxo1],
      publicMint: tokenCtx.mint,
      publicAmountSpl: splAmount,
      publicAmountSol: BN_0,
      hasher,
      relayerFee: BN_0,
      changeUtxoAccount: k0,
      action: Action.UNSHIELD,
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

  it("unshield SPL - with relayer fee", async () => {
    const outUtxos = createOutUtxos({
      inUtxos: [utxo1],
      publicMint: tokenCtx.mint,
      publicAmountSpl: splAmount,
      publicAmountSol: BN_0,
      hasher,
      relayerFee,
      changeUtxoAccount: k0,
      action: Action.UNSHIELD,
      numberMaxOutUtxos,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    assert.equal(
      outUtxos[0].amounts[0].toString(),
      utxo1.amounts[0].sub(relayerFee).toString(),
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

  it("unshield sol - no relayer fee", async () => {
    const outUtxos = createOutUtxos({
      inUtxos: [utxo1],
      publicMint: tokenCtx.mint,
      publicAmountSpl: BN_0,
      publicAmountSol: solAmount,
      hasher,
      relayerFee: BN_0,
      changeUtxoAccount: k0,
      action: Action.UNSHIELD,
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

  it("unshield sol - with relayer fee", async () => {
    const outUtxos = createOutUtxos({
      inUtxos: [utxo1],
      publicMint: tokenCtx.mint,
      publicAmountSpl: BN_0,
      publicAmountSol: solAmount,
      hasher,
      relayerFee,
      changeUtxoAccount: k0,
      action: Action.UNSHIELD,
      numberMaxOutUtxos,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    assert.equal(
      outUtxos[0].amounts[0].toString(),
      utxo1.amounts[0].sub(relayerFee).sub(solAmount).toString(),
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

  it("unshield spl & sol - no relayer fee", async () => {
    const outUtxos = createOutUtxos({
      inUtxos: [utxo1],
      publicMint: tokenCtx.mint,
      publicAmountSpl: splAmount,
      publicAmountSol: solAmount,
      hasher,
      relayerFee: BN_0,
      changeUtxoAccount: k0,
      action: Action.UNSHIELD,
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

  it("unshield spl & sol - with relayer fee", async () => {
    const outUtxos = createOutUtxos({
      inUtxos: [utxo1],
      publicMint: tokenCtx.mint,
      publicAmountSpl: splAmount,
      publicAmountSol: solAmount,
      hasher,
      relayerFee,
      changeUtxoAccount: k0,
      action: Action.UNSHIELD,
      numberMaxOutUtxos,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    assert.equal(
      outUtxos[0].amounts[0].toString(),
      utxo1.amounts[0].sub(relayerFee).sub(solAmount).toString(),
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

  it("unshield in:1SOL + 1SPL should merge 2-1", async () => {
    const outUtxos = createOutUtxos({
      publicMint: tokenCtx.mint,
      publicAmountSpl: splAmount,
      inUtxos: [utxo1, utxoSol],
      publicAmountSol: BN_0,
      hasher,
      changeUtxoAccount: k0,
      action: Action.UNSHIELD,
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

  it("unshield in:1SPL + 1SPL should merge 2-1", async () => {
    const outUtxos = createOutUtxos({
      publicMint: tokenCtx.mint,
      publicAmountSpl: splAmount,
      inUtxos: [utxo1, utxo1],
      publicAmountSol: BN_0,
      hasher,
      changeUtxoAccount: k0,
      action: Action.UNSHIELD,
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
      hasher,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    outUtxos = createOutUtxos({
      publicMint: tokenCtx.mint,
      publicAmountSpl: BN_0,
      inUtxos: [utxo1],
      outUtxos,
      relayerFee,
      publicAmountSol: BN_0,
      hasher,
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
        relayerFee.toNumber() -
        outUtxos[0].amounts[0].toNumber(),
      `${outUtxos[1].amounts[0]} fee != ${
        utxo1.amounts[0].toNumber() -
        relayerFee.toNumber() -
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
    const hasher = await WasmHasher.getInstance();

    const mint = MINT;
    const account1 = Account.createFromSeed(hasher, seed32);
    const account2 = Account.createFromSeed(
      hasher,
      new Uint8Array(32).fill(4).toString(),
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
      hasher,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
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
  let hasher: Hasher,
    assetPubkey: PublicKey,
    inUtxos: [Utxo, Utxo],
    lightProvider: Provider;
  before(async () => {
    lightProvider = await Provider.loadMock();
    hasher = await WasmHasher.getInstance();
    assetPubkey = new PublicKey(0);
    inUtxos = [
      createUtxo(hasher, [new BN(5)], [assetPubkey]),
      createUtxo(hasher, [new BN(3)], [assetPubkey]),
    ];
  });
  // Helper function to create a UTXO with specific amounts and assets
  function createUtxo(
    hasher: Hasher,
    amounts: BN[],
    assets: PublicKey[],
  ): Utxo {
    return new Utxo({
      hasher,
      amounts,
      assets,
      blinding: BN_0,
      publicKey: Account.random(hasher).keypair.publicKey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
  }

  it("should not throw an error if input UTXOs sum is equal to output UTXOs sum", () => {
    const outUtxos = [createUtxo(hasher, [new BN(8)], [assetPubkey])];

    expect(() =>
      validateUtxoAmounts({ assetPubkeys: [assetPubkey], inUtxos, outUtxos }),
    ).not.to.throw();
  });

  it("should not throw an error if input UTXOs sum is greater than output UTXOs sum", () => {
    const outUtxos = [createUtxo(hasher, [new BN(7)], [assetPubkey])];

    expect(() =>
      validateUtxoAmounts({ assetPubkeys: [assetPubkey], inUtxos, outUtxos }),
    ).not.to.throw();
  });

  it("should throw an error if input UTXOs sum is less than output UTXOs sum", () => {
    const outUtxos = [createUtxo(hasher, [new BN(9)], [assetPubkey])];

    expect(() =>
      validateUtxoAmounts({ assetPubkeys: [assetPubkey], inUtxos, outUtxos }),
    ).to.throw(CreateUtxoError);
  });
});

describe("Test createOutUtxos Errors", () => {
  let hasher: Hasher, k0: Account;

  let splAmount: BN,
    token,
    tokenCtx: TokenData,
    utxo1: Utxo,
    utxoSol: Utxo,
    lightProvider: Provider;
  before(async () => {
    lightProvider = await Provider.loadMock();
    hasher = await WasmHasher.getInstance();
    k0 = Account.createFromSeed(hasher, seed32);
    splAmount = new BN(3);
    token = "USDC";
    const tmpTokenCtx = TOKEN_REGISTRY.get(token);
    if (!tmpTokenCtx) throw new Error("Token not supported!");
    tokenCtx = tmpTokenCtx as TokenData;
    splAmount = splAmount.mul(new BN(tokenCtx.decimals));
    utxo1 = new Utxo({
      hasher,
      assets: [SystemProgram.programId, tokenCtx.mint],
      amounts: [new BN(1e8), new BN(5 * tokenCtx.decimals.toNumber())],
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      publicKey: k0.keypair.publicKey,
    });
    utxoSol = new Utxo({
      hasher,
      assets: [SystemProgram.programId],
      amounts: [new BN(1e6)],
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      publicKey: k0.keypair.publicKey,
    });

    createOutUtxos({
      publicMint: tokenCtx.mint,
      publicAmountSpl: splAmount,
      inUtxos: [utxo1, utxoSol],
      publicAmountSol: BN_0,
      changeUtxoAccount: k0,
      action: Action.UNSHIELD,
      hasher,
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
        action: Action.UNSHIELD,
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
        hasher,
        changeUtxoAccount: k0,
        action: Action.UNSHIELD,
        outUtxos: [
          new Utxo({
            hasher,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            publicKey: k0.keypair.publicKey,
          }),
          new Utxo({
            hasher,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            publicKey: k0.keypair.publicKey,
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
        hasher,
        changeUtxoAccount: k0,
        action: Action.UNSHIELD,
        outUtxos: [
          new Utxo({
            hasher,
            assets: [SystemProgram.programId, invalidMint],
            amounts: [BN_0, BN_1],
            assetLookupTable: [
              ...lightProvider.lookUpTables.assetLookupTable,
              ...[invalidMint.toBase58()],
            ],
            publicKey: k0.keypair.publicKey,
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
        hasher,
        changeUtxoAccount: k0,
        action: Action.UNSHIELD,
        outUtxos: [
          new Utxo({
            hasher,
            assets: [SystemProgram.programId, utxo1.assets[1]],
            amounts: [BN_0, new BN(1e12)],
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            publicKey: k0.keypair.publicKey,
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
        hasher,
        changeUtxoAccount: k0,
        action: Action.UNSHIELD,
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
        hasher,
        changeUtxoAccount: k0,
        action: Action.UNSHIELD,
        relayerFee: BN_1,
      });
    })
      .to.throw(CreateUtxoError)
      .includes({
        code: CreateUtxoErrorCode.NO_PUBLIC_MINT_PROVIDED,
        functionName: "createOutUtxos",
      });
  });

  // it("SPL_AMOUNT_UNDEFINED",async () => {
  //     expect(()=>{
  //         // @ts-ignore
  //         createOutUtxos({
  //             publicMint: tokenCtx.mint,
  //             publicAmountSpl: splAmount,
  //             inUtxos: [utxo1, utxoSol],
  //             publicAmountSol: BN_0,
  //             poseidon,
  //             changeUtxoAccount: k0,
  //             action: Action.UNSHIELD,
  //             // @ts-ignore
  //             recipients: [{account: recipientAccount, mint: utxo1.assets[1], solAmount: BN_0}],
  //         });
  //     }).to.throw(CreateUtxoError).includes({
  //         code: CreateUtxoErrorCode.SPL_AMOUNT_UNDEFINED,
  //         functionName: "createOutUtxos"
  //     })
  // })

  it("INVALID_OUTPUT_UTXO_LENGTH", async () => {
    const invalidMint = SolanaKeypair.generate().publicKey;

    const utxoSol0 = new Utxo({
      hasher,
      assets: [SystemProgram.programId, invalidMint],
      amounts: [new BN(1e6), new BN(1e6)],
      assetLookupTable: [
        ...lightProvider.lookUpTables.assetLookupTable,
        ...[invalidMint.toBase58()],
      ],
      publicKey: k0.keypair.publicKey,
    });

    expect(() => {
      createOutUtxos({
        publicMint: tokenCtx.mint,
        publicAmountSpl: splAmount,
        inUtxos: [utxo1, utxoSol0],
        publicAmountSol: BN_0,
        hasher,
        changeUtxoAccount: k0,
        action: Action.UNSHIELD,
        outUtxos: [
          new Utxo({
            hasher,
            assets: [SystemProgram.programId, utxo1.assets[1]],
            amounts: [BN_0, BN_1],
            assetLookupTable: [
              ...lightProvider.lookUpTables.assetLookupTable,
              ...[invalidMint.toBase58()],
            ],
            publicKey: k0.keypair.publicKey,
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

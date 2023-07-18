import * as anchor from "@coral-xyz/anchor";

import {
  Utxo,
  ADMIN_AUTH_KEYPAIR,
  KEYPAIR_PRIVKEY,
  Account,
  Provider as LightProvider,
  confirmConfig,
  createAccountObject,
} from "@lightprotocol/zk.js";
import { SystemProgram, PublicKey } from "@solana/web3.js";

import { buildPoseidonOpt } from "circomlibjs";
import { BN } from "@coral-xyz/anchor";
import { it } from "mocha";
import { IDL } from "../target/types/mock_verifier";
import { assert, expect } from "chai";

var RELAYER: any;

describe("Mock verifier functional", () => {
  // Configure the client to use the local cluster.
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  const provider = anchor.AnchorProvider.local(
    "http://127.0.0.1:8899",
    confirmConfig,
  );
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

  anchor.setProvider(provider);
  var poseidon: any, lightProvider: LightProvider;
  before(async () => {
    lightProvider = await LightProvider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
    });
    poseidon = await buildPoseidonOpt();
    new Account({
      poseidon,
      seed: KEYPAIR_PRIVKEY.toString(),
    });
  });

  var outputUtxo: Utxo;
  it("To from bytes ", async () => {
    const account = new Account({
      poseidon,
      seed: new Array(32).fill(1).toString(),
    });
    outputUtxo = new Utxo({
      poseidon,
      assets: [SystemProgram.programId],
      account,
      amounts: [new BN(1_000_000)],
      appData: { testInput1: new BN(1), testInput2: new BN(1) },
      appDataIdl: IDL,
      verifierAddress: new PublicKey(
        lightProvider.lookUpTables.assetLookupTable[1],
      ),
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
      appDataIdl: IDL,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    Utxo.equal(poseidon, outputUtxo, utxo1);
  });

  it("Pick app data from utxo data", () => {
    let data = createAccountObject(
      {
        testInput1: 1,
        testInput2: 2,
        rndOtherStuff: { s: 2342 },
        o: [2, 2, new BN(2)],
      },
      IDL.accounts,
      "utxoAppData",
    );
    assert.equal(data.testInput1, 1);
    assert.equal(data.testInput2, 2);
    assert.equal(data.rndOtherStuff, undefined);
    assert.equal(data.o, undefined);

    expect(() => {
      createAccountObject(
        { testInput1: 1, rndOtherStuff: { s: 2342 }, o: [2, 2, new BN(2)] },
        IDL.accounts,
        "utxoAppData",
      );
    }).to.throw(Error);
  });
});

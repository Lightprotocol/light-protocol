import * as anchor from "@project-serum/anchor";

import {
  Utxo,
  Transaction,
  ADMIN_AUTH_KEYPAIR,
  initLookUpTableFromFile,
  setUpMerkleTree,
  createTestAccounts,
  KEYPAIR_PRIVKEY,
  Account,
  MERKLE_TREE_KEY,
  TransactionParameters,
  Provider as LightProvider,
  userTokenAccount,
  ADMIN_AUTH_KEY,
  VerifierTwo,
  confirmConfig,
  Action,
  TestRelayer,
  hashAndTruncateToCircuit,
  createAccountObject
} from "light-sdk";
import {
  Keypair as SolanaKeypair,
  SystemProgram,
  PublicKey,
} from "@solana/web3.js";
import { marketPlaceVerifierProgramId, MockVerifier,  } from "../sdk/src/index";

import { buildPoseidonOpt } from "circomlibjs";
import { BN } from "@project-serum/anchor";
import { it } from "mocha";
import { IDL } from "../target/types/mock_verifier";
import { assert, expect } from "chai";

var POSEIDON, LOOK_UP_TABLE,RELAYER,KEYPAIR, relayerRecipientSol: PublicKey;


describe("Mock verifier functional", () => {
  // Configure the client to use the local cluster.
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  const provider = anchor.AnchorProvider.local(
    "http://127.0.0.1:8899",
    confirmConfig,
  );
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

  anchor.setProvider(provider);
  var poseidon
  before(async () => {
    poseidon = await buildPoseidonOpt();
    KEYPAIR = new Account({
      poseidon,
      seed: KEYPAIR_PRIVKEY.toString(),
    });
  });

  var outputUtxo;
  it("To from bytes ",async () => {
    const account = new Account({
      poseidon,
      seed: new Array(32).fill(1).toString(),
    });
    outputUtxo = new Utxo({
      poseidon,
      assets: [SystemProgram.programId],
      account,
      amounts: [new BN(1_000_000)],
      appData: {testInput1: new BN(1), testInput2: new BN(1)},
      appDataIdl: IDL,
      verifierAddress: marketPlaceVerifierProgramId,
      index: 0
    });
    let bytes = await outputUtxo.toBytes();
    
    let utxo1 = Utxo.fromBytes({poseidon, bytes,index: 0, account, appDataIdl: IDL});
    Utxo.equal(poseidon, outputUtxo, utxo1);
  })

  it("Pick app data from utxo data", ()=> {
    let data = createAccountObject({testInput1: 1, testInput2: 2, rndOtherStuff: {s:2342}, o: [2,2,new BN(2)]},IDL.accounts,  "utxoAppData");
    assert.equal(data.testInput1, 1);
    assert.equal(data.testInput2, 2);
    assert.equal(data.rndOtherStuff, undefined);
    assert.equal(data.o, undefined);

    expect(()=>{
        createAccountObject({testInput1: 1, rndOtherStuff: {s:2342}, o: [2,2,new BN(2)]},IDL.accounts, "utxoAppData")
    }).to.throw(Error)
  })

})
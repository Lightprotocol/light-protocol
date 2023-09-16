//@ts-nocheck
import { assert, expect } from "chai";
import { it } from "mocha";
import { Prover } from "@lightprotocol/prover.js";
import * as anchor from "@coral-xyz/anchor";
let circomlibjs = require("circomlibjs");
import { Keypair as SolanaKeypair } from "@solana/web3.js";
import { utils } from "ffjavascript";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
chai.use(chaiAsPromised);

import {
  FEE_ASSET,
  Provider as LightProvider,
  MINT,
  Transaction,
  TransactionParameters,
  Action,
  Utxo,
  Account,
  MerkleTree,
  IDL_VERIFIER_PROGRAM_ZERO,
} from "../../light-zk.js/src";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

describe("Prover Functionality Tests", () => {
  const depositAmount = 20_000;
  const depositFeeAmount = 10_000;

  const mockPubkey = SolanaKeypair.generate().publicKey;
  const mockPubkey2 = SolanaKeypair.generate().publicKey;

  let lightProvider: LightProvider;
  let paramsDeposit: TransactionParameters;
  let deposit_utxo: Utxo;
  let keypair: Account;
  let poseidon: any;
  before(async () => {
    poseidon = await circomlibjs.buildPoseidonOpt();
    lightProvider = await LightProvider.loadMock();

    deposit_utxo = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
      blinding: new anchor.BN(new Array(31).fill(1)),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    paramsDeposit = new TransactionParameters({
      outputUtxos: [deposit_utxo],
      eventMerkleTreePubkey: mockPubkey2,
      transactionMerkleTreePubkey: mockPubkey2,
      poseidon,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet?.publicKey,
      action: Action.SHIELD,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });

    lightProvider.solMerkleTree!.merkleTree = new MerkleTree(18, poseidon, [
      deposit_utxo.getCommitment(poseidon),
    ]);

    assert.equal(
      lightProvider.solMerkleTree?.merkleTree.indexOf(
        deposit_utxo.getCommitment(poseidon),
      ),
      0,
    );
  });

  after(async () => {
    globalThis.curve_bn128.terminate();
  });

  it("Verifies Prover with VerifierZero", async () => {
    let tx = new Transaction({
      provider: lightProvider,
      params: paramsDeposit,
    });

    await tx.compile();

    const genericProver = new Prover(tx.params.verifierIdl, tx.firstPath);
    await genericProver.addProofInputs(tx.proofInput);
    await genericProver.fullProve();
    await tx.getProof();

    const publicInputsBytes = genericProver.parseToBytesArray(
      genericProver.publicInputs,
    );
    const publicInputsJson = JSON.stringify(
      genericProver.publicInputs,
      null,
      1,
    );

    const publicInputsBytesJson = JSON.parse(publicInputsJson.toString());
    const publicInputsBytesVerifier = new Array<Array<number>>();
    for (let i in publicInputsBytesJson) {
      let ref: Array<number> = Array.from([
        ...utils.leInt2Buff(
          utils.unstringifyBigInts(publicInputsBytesJson[i]),
          32,
        ),
      ]).reverse();
      publicInputsBytesVerifier.push(ref);
    }

    expect(publicInputsBytes).to.deep.equal(publicInputsBytesVerifier);
  });

  it("Checks identical public inputs with different randomness", async () => {
    let tx = new Transaction({
      provider: lightProvider,
      params: paramsDeposit,
    });

    await tx.compile();

    const prover1 = new Prover(tx.params.verifierIdl, tx.firstPath);
    await prover1.addProofInputs(tx.proofInput);
    await prover1.fullProve();
    await tx.getProof();

    const prover2 = new Prover(tx.params.verifierIdl, tx.firstPath);
    await prover2.addProofInputs(tx.proofInput);
    await prover2.fullProve();
    await tx.getProof();

    expect(prover1.publicInputs).to.deep.equal(
      prover2.publicInputs,
      "Public inputs should be the same for different proofs with identical inputs",
    );
  });
});

import { assert, expect } from "chai";
import { it } from "mocha";
import { Prover } from "../src";
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

describe("Test Prover Functional", () => {
  const seed32 = new Uint8Array(32).fill(1).toString();
  const depositAmount = 20_000;
  const depositFeeAmount = 10_000;

  const mockPubkey = SolanaKeypair.generate().publicKey;
  const mockPubkey2 = SolanaKeypair.generate().publicKey;

  let lightProvider: LightProvider;
  let paramsDeposit: TransactionParameters;
  let deposit_utxo: Utxo;
  let keypair: Account;

  before(async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    lightProvider = await LightProvider.loadMock();

    deposit_utxo = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [
        new anchor.BN(depositFeeAmount),
        new anchor.BN(depositAmount),
      ],
      account: keypair,
      blinding: new anchor.BN(new Array(31).fill(1)),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    paramsDeposit = new TransactionParameters({
      outputUtxos: [deposit_utxo],
      transactionMerkleTreePubkey: mockPubkey2,
      lookUpTable: lightProvider.lookUpTable,
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

  it("Test Prover for VerifierZero", async () => {
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

  it("Test repeated proving with different randomness returns identical public inputs", async () => {
    let tx = new Transaction({
      provider: lightProvider,
      params: paramsDeposit,
    });

    await tx.compile();

    const genericProver1 = new Prover(tx.params.verifierIdl, tx.firstPath);
    await genericProver1.addProofInputs(tx.proofInput);
    await genericProver1.fullProve();
    await tx.getProof();

    const publicInputs1 = genericProver1.publicInputs;

    // Generate a new proof with different randomness
    const genericProver2 = new Prover(tx.params.verifierIdl, tx.firstPath);
    await genericProver2.addProofInputs(tx.proofInput);
    await genericProver2.fullProve();
    await tx.getProof();

    const publicInputs2 = genericProver2.publicInputs;

    expect(publicInputs1).to.deep.equal(
      publicInputs2,
      "Public inputs should be the same for different proofs with identical inputs",
    );
  });
});

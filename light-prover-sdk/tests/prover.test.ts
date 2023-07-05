import { assert, expect } from "chai";
import { it } from "mocha";
import { Prover } from "../src/prover";
import * as anchor from "@coral-xyz/anchor";
import * as circomlibjs from "circomlibjs";
import { Keypair as SolanaKeypair } from "@solana/web3.js";
import { utils } from "ffjavascript";

import {
  FEE_ASSET,
  Provider as LightProvider,
  MINT,
  Transaction,
  TransactionParameters,
  Action,
  Utxo,
  MerkleTree,
  IDL_VERIFIER_PROGRAM_ZERO,
} from "../../light-zk.js/src";
import { ProofInputs } from "../src/generics";
import {IDL_VERIFIER_PROGRAM_ONE} from "../../light-zk.js";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

const DEPOSIT_AMOUNT = 20_000;
const DEPOSIT_FEE_AMOUNT = 10_000;
describe("Test Prover Functional", () => {
  let lightProvider: LightProvider;
  let paramsDeposit: TransactionParameters;
  let deposit_utxo: Utxo;
  let mockPubkey: anchor.web3.PublicKey;

  before(async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    lightProvider = await LightProvider.loadMock();

    deposit_utxo = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(DEPOSIT_FEE_AMOUNT), new anchor.BN(DEPOSIT_AMOUNT)],
      blinding: new anchor.BN(new Array(31).fill(1)),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    mockPubkey = SolanaKeypair.generate().publicKey;
    const mockPubkey2 = SolanaKeypair.generate().publicKey;

    paramsDeposit = new TransactionParameters({
      outputUtxos: [deposit_utxo],
      transactionMerkleTreePubkey: mockPubkey2,
      lookUpTable: lightProvider.lookUpTable,
      poseidon: poseidon,
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

  after( async () => {
    globalThis.curve_bn128.terminate();
  });

  it("Test Generic Self-contained Prover for VerifierZero", async () => {
    let tx = new Transaction({
      provider: lightProvider,
      params: paramsDeposit,
    })

    await tx.compile();

    const genericProver = new Prover(tx.params.verifierIdl, tx.firstPath);
    await genericProver.addProofInputs(tx.proofInput);
    await genericProver.fullProve();
    await tx.getProof();

    const publicInputsBytes = genericProver.parseToBytesArray(genericProver.publicInputs);
    const publicInputsJson = JSON.stringify(genericProver.publicInputs, null, 1);

    const publicInputsBytesJson = JSON.parse(publicInputsJson.toString());
    const publicInputsBytesVerifier = new Array<Array<number>>();
    for (let i in publicInputsBytesJson) {
      let ref: Array<number> = Array.from([
        ...utils.leInt2Buff(utils.unstringifyBigInts(publicInputsBytesJson[i]), 32),
      ]).reverse();
      publicInputsBytesVerifier.push(ref);
    }

    expect(publicInputsBytes).to.deep.equal(publicInputsBytesVerifier);
  });

  it("Test repeated proving with different randomness returns identical public inputs", async () => {
    let tx = new Transaction({
      provider: lightProvider,
      params: paramsDeposit,
    })

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

    expect(publicInputs1).to.deep.equal(publicInputs2, "Public inputs should be the same for different proofs with identical inputs");
  });

});


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

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

const DEPOSIT_AMOUNT = 20_000;
const DEPOSIT_FEE_AMOUNT = 10_000;
describe("Test Prover Functional", () => {
  let lightProvider: LightProvider;
  let paramsDeposit: TransactionParameters;

  before(async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    lightProvider = await LightProvider.loadMock();

    const deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(DEPOSIT_FEE_AMOUNT), new anchor.BN(DEPOSIT_AMOUNT)],
      blinding: new anchor.BN(new Array(31).fill(1)),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    const mockPubkey = SolanaKeypair.generate().publicKey;
    const mockPubkey2 = SolanaKeypair.generate().publicKey;

    paramsDeposit = new TransactionParameters({
      outputUtxos: [deposit_utxo1],
      transactionMerkleTreePubkey: mockPubkey2,
      lookUpTable: lightProvider.lookUpTable,
      poseidon: poseidon,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet?.publicKey,
      action: Action.SHIELD,
      transactionNonce: 0,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });

    lightProvider.solMerkleTree!.merkleTree = new MerkleTree(18, poseidon, [
      deposit_utxo1.getCommitment(poseidon),
    ]);

    assert.equal(
      lightProvider.solMerkleTree?.merkleTree.indexOf(
        deposit_utxo1.getCommitment(poseidon),
      ),
      0,
    );
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
  })
});


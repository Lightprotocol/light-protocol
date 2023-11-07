import { assert, expect } from "chai";
import { it } from "mocha";
import { Prover } from "@lightprotocol/prover.js";
import * as anchor from "@coral-xyz/anchor";
const circomlibjs = require("circomlibjs");
import { Keypair as SolanaKeypair } from "@solana/web3.js";
const ffjavascript = require("ffjavascript");
const utils = ffjavascript.utils;

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
  IDL_LIGHT_PSP2IN2OUT,
} from "../../zk.js/src";
import { MerkleTree } from "@lightprotocol/circuit-lib.js";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
process.env.LIGHT_PROTOCOL_ATOMIC_TRANSACTIONS = "true";

describe("Prover Functionality Tests", () => {
  const shieldAmount = 20_000;
  const shieldFeeAmount = 10_000;

  const mockPubkey = SolanaKeypair.generate().publicKey;
  const mockPubkey2 = SolanaKeypair.generate().publicKey;

  let lightProvider: LightProvider;
  let paramsShield: TransactionParameters;
  let shieldUtxo: Utxo;
  let account: Account;
  let poseidon: any;
  before(async () => {
    poseidon = await circomlibjs.buildPoseidonOpt();
    lightProvider = await LightProvider.loadMock();
    account = new Account({ poseidon });

    shieldUtxo = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(shieldFeeAmount), new anchor.BN(shieldAmount)],
      publicKey: account.pubkey,
      blinding: new anchor.BN(new Array(31).fill(1)),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    paramsShield = new TransactionParameters({
      outputUtxos: [shieldUtxo],
      eventMerkleTreePubkey: mockPubkey2,
      transactionMerkleTreePubkey: mockPubkey2,
      poseidon,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet?.publicKey,
      action: Action.SHIELD,
      verifierIdl: IDL_LIGHT_PSP2IN2OUT,
      account,
    });

    lightProvider.solMerkleTree!.merkleTree = new MerkleTree(18, poseidon, [
      shieldUtxo.getCommitment(poseidon),
    ]);

    assert.equal(
      lightProvider.solMerkleTree?.merkleTree.indexOf(
        shieldUtxo.getCommitment(poseidon),
      ),
      0,
    );
  });

  after(async () => {
    //@ts-ignore
    globalThis.curve_bn128.terminate();
  });

  it("Verifies Prover with VerifierZero", async () => {
    const tx = new Transaction({
      ...(await lightProvider.getRootIndex()),
      solMerkleTree: lightProvider.solMerkleTree!,
      params: paramsShield,
    });

    await tx.compile(lightProvider.poseidon, account);

    const genericProver = new Prover(tx.params.verifierIdl, tx.firstPath);
    tx.proofInput["inPrivateKey"] = new Array(2).fill(account.privkey);
    await genericProver.addProofInputs(tx.proofInput);
    await genericProver.fullProve();
    await tx.getProof(account);

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
    for (const i in publicInputsBytesJson) {
      const ref: Array<number> = Array.from([
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
    const tx = new Transaction({
      ...(await lightProvider.getRootIndex()),
      solMerkleTree: lightProvider.solMerkleTree!,
      params: paramsShield,
    });

    await tx.compile(lightProvider.poseidon, account);

    const prover1 = new Prover(tx.params.verifierIdl, tx.firstPath);
    tx.proofInput["inPrivateKey"] = new Array(2).fill(account.privkey);
    await prover1.addProofInputs(tx.proofInput);
    await prover1.fullProve();
    await tx.getProof(account);

    const prover2 = new Prover(tx.params.verifierIdl, tx.firstPath);
    await prover2.addProofInputs(tx.proofInput);
    await prover2.fullProve();
    await tx.getProof(account);

    expect(prover1.publicInputs).to.deep.equal(
      prover2.publicInputs,
      "Public inputs should be the same for different proofs with identical inputs",
    );
  });
});

import { assert, expect } from "chai";
import { it } from "mocha";
import { Prover } from "@lightprotocol/prover.js";
import * as anchor from "@coral-xyz/anchor";

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
  Utxo,
  Account,
  IDL_LIGHT_PSP2IN2OUT,
  ShieldTransaction,
  createShieldTransaction,
  ShieldTransactionInput,
  MerkleTreeConfig,
  getVerifierProgramId,
  createSystemProofInputs,
  getSystemProof,
} from "../../zk.js/src";
import { WasmHasher, Hasher } from "@lightprotocol/account.rs";
import { MerkleTree } from "@lightprotocol/circuit-lib.js";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

describe("Prover Functionality Tests", () => {
  const shieldAmount = 20_000;
  const shieldFeeAmount = 10_000;
  const verifierIdl = IDL_LIGHT_PSP2IN2OUT;
  const mockPubkey = SolanaKeypair.generate().publicKey;
  const mockPubkey2 = SolanaKeypair.generate().publicKey;
  const path = require("path");
  const firstPath = path.resolve(__dirname, "../build-circuits/");
  let lightProvider: LightProvider;
  let shieldUtxo: Utxo;
  let account: Account;
  let hasher: Hasher;
  let shieldTransaction: ShieldTransaction;
  let merkleTree: MerkleTree;
  before(async () => {
    hasher = await WasmHasher.getInstance();
    lightProvider = await LightProvider.loadMock();
    account = new Account({ hasher });

    shieldUtxo = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(shieldFeeAmount), new anchor.BN(shieldAmount)],
      publicKey: account.pubkey,
      blinding: new anchor.BN(new Array(31).fill(1)),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    const shieldTransactionInput: ShieldTransactionInput = {
      hasher,
      mint: MINT,
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      senderSpl: mockPubkey,
      signer: mockPubkey2,
      systemPspId: getVerifierProgramId(IDL_LIGHT_PSP2IN2OUT),
      account,
      outputUtxos: [shieldUtxo],
    };

    shieldTransaction = await createShieldTransaction(shieldTransactionInput);

    merkleTree = new MerkleTree(18, hasher, [shieldUtxo.getCommitment(hasher)]);

    assert.equal(merkleTree.indexOf(shieldUtxo.getCommitment(hasher)), 0);
  });

  after(async () => {
    //@ts-ignore
    globalThis.curve_bn128.terminate();
  });

  it("Verifies Prover with VerifierZero", async () => {
    const systemProofInputs = createSystemProofInputs({
      transaction: shieldTransaction,
      hasher,
      account,
      root: merkleTree.root(),
    });

    const genericProver = new Prover(verifierIdl, firstPath);
    systemProofInputs["inPrivateKey"] = new Array(2).fill(account.privkey);
    await genericProver.addProofInputs(systemProofInputs);
    await genericProver.fullProve();
    await getSystemProof({
      account,
      inputUtxos: shieldTransaction.private.inputUtxos,
      verifierIdl,
      systemProofInputs,
    });

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
    const proofInput = createSystemProofInputs({
      transaction: shieldTransaction,
      hasher,
      account,
      root: merkleTree.root(),
    });

    const prover1 = new Prover(verifierIdl, firstPath);
    proofInput["inPrivateKey"] = new Array(2).fill(account.privkey);
    await prover1.addProofInputs(proofInput);
    await prover1.fullProve();
    await getSystemProof({
      account,
      inputUtxos: shieldTransaction.private.inputUtxos,
      verifierIdl,
      systemProofInputs: proofInput,
    });

    const prover2 = new Prover(verifierIdl, firstPath);
    await prover2.addProofInputs(proofInput);
    await prover2.fullProve();
    await getSystemProof({
      account,
      inputUtxos: shieldTransaction.private.inputUtxos,
      verifierIdl,
      systemProofInputs: proofInput,
    });

    expect(prover1.publicInputs).to.deep.equal(
      prover2.publicInputs,
      "Public inputs should be the same for different proofs with identical inputs",
    );
  });
});

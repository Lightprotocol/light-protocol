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
  Account,
  IDL_LIGHT_PSP2IN2OUT,
  CompressTransaction,
  createCompressTransaction,
  CompressTransactionInput,
  MERKLE_TREE_SET,
  getVerifierProgramId,
  createSystemProofInputs,
  getSystemProof,
  createOutUtxo,
  OutUtxo,
} from "../../zk.js/src";
import { WasmFactory, LightWasm } from "@lightprotocol/account.rs";
import { MerkleTree } from "@lightprotocol/circuit-lib.js";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

describe("Prover Functionality Tests", () => {
  const compressAmount = 20_000;
  const compressFeeAmount = 10_000;
  const verifierIdl = IDL_LIGHT_PSP2IN2OUT;
  const mockPubkey = SolanaKeypair.generate().publicKey;
  const mockPubkey2 = SolanaKeypair.generate().publicKey;
  const path = require("path");
  const firstPath = path.resolve(__dirname, "../build-circuits/");
  let lightProvider: LightProvider;
  let compressUtxo: OutUtxo;
  let account: Account;
  let lightWasm: LightWasm;
  let compressTransaction: CompressTransaction;
  let merkleTree: MerkleTree;
  before(async () => {
    lightWasm = await WasmFactory.getInstance();
    lightProvider = await LightProvider.loadMock();
    account = Account.random(lightWasm);

    compressUtxo = createOutUtxo({
      lightWasm,
      assets: [FEE_ASSET, MINT],
      amounts: [
        new anchor.BN(compressFeeAmount),
        new anchor.BN(compressAmount),
      ],
      owner: account.keypair.publicKey,
      blinding: new anchor.BN(new Array(31).fill(1)),
    });
    const compressTransactionInput: CompressTransactionInput = {
      lightWasm,
      mint: MINT,
      merkleTreeSetPubkey: MERKLE_TREE_SET,
      senderSpl: mockPubkey,
      signer: mockPubkey2,
      systemPspId: getVerifierProgramId(IDL_LIGHT_PSP2IN2OUT),
      account,
      outputUtxos: [compressUtxo],
    };

    compressTransaction = await createCompressTransaction(
      compressTransactionInput,
    );

    merkleTree = new MerkleTree(22, lightWasm, [compressUtxo.hash.toString()]);

    assert.equal(merkleTree.indexOf(compressUtxo.hash.toString()), 0);
  });

  after(async () => {
    //@ts-ignore
    globalThis.curve_bn128.terminate();
  });

  it("Verifies Prover with VerifierZero", async () => {
    const systemProofInputs = createSystemProofInputs({
      transaction: compressTransaction,
      lightWasm,
      account,
      root: merkleTree.root(),
    });

    const genericProver = new Prover(verifierIdl, firstPath);
    systemProofInputs["inPrivateKey"] = new Array(2).fill(
      account.keypair.privateKey,
    );
    await genericProver.addProofInputs(systemProofInputs);
    await genericProver.fullProve();
    await getSystemProof({
      account,
      inputUtxos: compressTransaction.private.inputUtxos,
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
      transaction: compressTransaction,
      lightWasm,
      account,
      root: merkleTree.root(),
    });

    const prover1 = new Prover(verifierIdl, firstPath);
    proofInput["inPrivateKey"] = new Array(2).fill(account.keypair.privateKey);
    await prover1.addProofInputs(proofInput);
    await prover1.fullProve();
    await getSystemProof({
      account,
      inputUtxos: compressTransaction.private.inputUtxos,
      verifierIdl,
      systemProofInputs: proofInput,
    });

    const prover2 = new Prover(verifierIdl, firstPath);
    await prover2.addProofInputs(proofInput);
    await prover2.fullProve();
    await getSystemProof({
      account,
      inputUtxos: compressTransaction.private.inputUtxos,
      verifierIdl,
      systemProofInputs: proofInput,
    });

    expect(prover1.publicInputs).to.deep.equal(
      prover2.publicInputs,
      "Public inputs should be the same for different proofs with identical inputs",
    );
  });
});

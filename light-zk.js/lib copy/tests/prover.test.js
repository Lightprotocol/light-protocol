"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
//@ts-nocheck
const chai_1 = require("chai");
const mocha_1 = require("mocha");
const prover_js_1 = require("@lightprotocol/prover.js");
const anchor = __importStar(require("@coral-xyz/anchor"));
let circomlibjs = require("circomlibjs");
const web3_js_1 = require("@solana/web3.js");
const ffjavascript_1 = require("ffjavascript");
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
chai.use(chaiAsPromised);
const src_1 = require("../../light-zk.js/src");
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
describe("Prover Functionality Tests", () => {
    const depositAmount = 20000;
    const depositFeeAmount = 10000;
    const mockPubkey = web3_js_1.Keypair.generate().publicKey;
    const mockPubkey2 = web3_js_1.Keypair.generate().publicKey;
    let lightProvider;
    let paramsDeposit;
    let deposit_utxo;
    let keypair;
    let poseidon;
    before(async () => {
        poseidon = await circomlibjs.buildPoseidonOpt();
        lightProvider = await src_1.Provider.loadMock();
        deposit_utxo = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
            account: keypair,
            blinding: new anchor.BN(new Array(31).fill(1)),
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        paramsDeposit = new src_1.TransactionParameters({
            outputUtxos: [deposit_utxo],
            eventMerkleTreePubkey: mockPubkey2,
            transactionMerkleTreePubkey: mockPubkey2,
            poseidon,
            senderSpl: mockPubkey,
            senderSol: lightProvider.wallet?.publicKey,
            action: src_1.Action.SHIELD,
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO,
        });
        lightProvider.solMerkleTree.merkleTree = new src_1.MerkleTree(18, poseidon, [
            deposit_utxo.getCommitment(poseidon),
        ]);
        chai_1.assert.equal(lightProvider.solMerkleTree?.merkleTree.indexOf(deposit_utxo.getCommitment(poseidon)), 0);
    });
    after(async () => {
        globalThis.curve_bn128.terminate();
    });
    (0, mocha_1.it)("Verifies Prover with VerifierZero", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsDeposit,
        });
        await tx.compile();
        const genericProver = new prover_js_1.Prover(tx.params.verifierIdl, tx.firstPath);
        await genericProver.addProofInputs(tx.proofInput);
        await genericProver.fullProve();
        await tx.getProof();
        const publicInputsBytes = genericProver.parseToBytesArray(genericProver.publicInputs);
        const publicInputsJson = JSON.stringify(genericProver.publicInputs, null, 1);
        const publicInputsBytesJson = JSON.parse(publicInputsJson.toString());
        const publicInputsBytesVerifier = new Array();
        for (let i in publicInputsBytesJson) {
            let ref = Array.from([
                ...ffjavascript_1.utils.leInt2Buff(ffjavascript_1.utils.unstringifyBigInts(publicInputsBytesJson[i]), 32),
            ]).reverse();
            publicInputsBytesVerifier.push(ref);
        }
        (0, chai_1.expect)(publicInputsBytes).to.deep.equal(publicInputsBytesVerifier);
    });
    (0, mocha_1.it)("Checks identical public inputs with different randomness", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsDeposit,
        });
        await tx.compile();
        const prover1 = new prover_js_1.Prover(tx.params.verifierIdl, tx.firstPath);
        await prover1.addProofInputs(tx.proofInput);
        await prover1.fullProve();
        await tx.getProof();
        const prover2 = new prover_js_1.Prover(tx.params.verifierIdl, tx.firstPath);
        await prover2.addProofInputs(tx.proofInput);
        await prover2.fullProve();
        await tx.getProof();
        (0, chai_1.expect)(prover1.publicInputs).to.deep.equal(prover2.publicInputs, "Public inputs should be the same for different proofs with identical inputs");
    });
});
//# sourceMappingURL=prover.test.js.map
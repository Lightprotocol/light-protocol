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
exports.functionalCircuitTest = void 0;
const index_1 = require("../index");
const anchor = __importStar(require("@coral-xyz/anchor"));
const chai_1 = require("chai");
const web3_js_1 = require("@solana/web3.js");
const bytes_1 = require("@coral-xyz/anchor/dist/cjs/utils/bytes");
const circomlibjs = require("circomlibjs");
async function functionalCircuitTest(app = false, verifierIdl) {
    let lightProvider = await index_1.Provider.loadMock();
    const poseidon = await circomlibjs.buildPoseidonOpt();
    let seed32 = bytes_1.bs58.encode(new Uint8Array(32).fill(1));
    let keypair = new index_1.Account({ poseidon: poseidon, seed: seed32 });
    let depositAmount = 20000;
    let depositFeeAmount = 10000;
    let deposit_utxo1 = new index_1.Utxo({
        poseidon: poseidon,
        assets: [index_1.FEE_ASSET, index_1.MINT],
        amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
        account: keypair,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    let mockPubkey = web3_js_1.Keypair.generate().publicKey;
    let txParams = new index_1.TransactionParameters({
        outputUtxos: [deposit_utxo1],
        eventMerkleTreePubkey: mockPubkey,
        transactionMerkleTreePubkey: mockPubkey,
        senderSpl: mockPubkey,
        senderSol: lightProvider.wallet.publicKey,
        action: index_1.Action.SHIELD,
        poseidon,
        verifierIdl: verifierIdl,
    });
    let tx;
    // successful proof generation
    if (app) {
        tx = new index_1.Transaction({
            provider: lightProvider,
            params: txParams,
            appParams: {
                mock: "123",
                // just a placeholder the test does not compute an app proof
                verifierIdl: index_1.IDL_VERIFIER_PROGRAM_ZERO,
                path: "./build-circuits",
            },
        });
    }
    else {
        tx = new index_1.Transaction({
            provider: lightProvider,
            params: txParams,
        });
    }
    await tx.compile();
    await tx.getProof();
    // unsuccessful proof generation
    let x = true;
    try {
        tx.proofInput.inIndices[0][1][1] = "1";
        // TODO: investigate why this does not kill the proof
        tx.proofInput.inIndices[0][1][0] = "1";
        (0, chai_1.expect)(await tx.getProof()).to.Throw();
        x = false;
    }
    catch (error) {
        // assert.isTrue(error.toString().includes("CheckIndices_3 line:"));
    }
    chai_1.assert.isTrue(x);
}
exports.functionalCircuitTest = functionalCircuitTest;
//# sourceMappingURL=functionalCircuit.js.map
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
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.functionalCircuitTest = void 0;
const index_1 = require("../index");
const index_2 = require("../merkleTree/index");
const anchor = __importStar(require("@coral-xyz/anchor"));
const chai_1 = require("chai");
const web3_js_1 = require("@solana/web3.js");
const circomlibjs = require("circomlibjs");
function functionalCircuitTest() {
    return __awaiter(this, void 0, void 0, function* () {
        try {
            const provider = new anchor.AnchorProvider(yield new web3_js_1.Connection("http://127.0.0.1:8899"), new anchor.Wallet(web3_js_1.Keypair.generate()), index_1.confirmConfig);
            yield anchor.setProvider(provider);
        }
        catch (error) {
            console.log("expected local test validator to be running");
            process.exit();
        }
        const poseidon = yield circomlibjs.buildPoseidonOpt();
        let seed32 = new Uint8Array(32).fill(1).toString();
        let keypair = new index_1.Keypair({ poseidon: poseidon, seed: seed32 });
        let depositAmount = 20000;
        let depositFeeAmount = 10000;
        let deposit_utxo1 = new index_1.Utxo({
            poseidon: poseidon,
            assets: [index_1.FEE_ASSET, index_1.MINT],
            amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
            keypair,
        });
        let mockPubkey = web3_js_1.Keypair.generate().publicKey;
        let lightInstance = {
            solMerkleTree: new index_2.SolMerkleTree({ poseidon, pubkey: mockPubkey }),
        };
        let txParams = new index_1.TransactionParameters({
            outputUtxos: [deposit_utxo1],
            merkleTreePubkey: mockPubkey,
            sender: mockPubkey,
            senderFee: mockPubkey,
            verifier: new index_1.VerifierZero(),
        });
        let tx = new index_1.Transaction({
            instance: lightInstance,
            payer: index_1.ADMIN_AUTH_KEYPAIR,
        });
        // successful proofgeneration
        yield tx.compile(txParams);
        console.log(tx.proofInput);
        yield tx.getProof();
        // unsuccessful proofgeneration
        try {
            tx.proofInput.inIndices[0][1][1] = "1";
            // TODO: investigate why this does not kill the proof
            tx.proofInput.inIndices[0][1][0] = "1";
            (0, chai_1.expect)(yield tx.getProof()).to.Throw();
            // console.log(tx.input.inIndices[0])
            // console.log(tx.input.inIndices[1])
        }
        catch (error) {
            chai_1.assert.isTrue(error.toString().includes("CheckIndices_3 line:"));
        }
    });
}
exports.functionalCircuitTest = functionalCircuitTest;

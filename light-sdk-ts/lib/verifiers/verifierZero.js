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
exports.VerifierZero = void 0;
const anchor = __importStar(require("@coral-xyz/anchor"));
const anchor_1 = require("@coral-xyz/anchor");
const index_1 = require("../index");
const verifier_program_zero_1 = require("../idls/verifier_program_zero");
// Proofgen does not work within sdk needs circuit-build
// TODO: bundle files in npm package
// TODO: define verifier with an Idl thus absorb this functionality into the Transaction class
class VerifierZero {
    constructor() {
        try {
            this.verifierProgram = new anchor_1.Program(verifier_program_zero_1.VerifierProgramZero, index_1.verifierProgramZeroProgramId);
        }
        catch (error) { }
        // ./build-circuits/transactionMasp2_js/
        this.wtnsGenPath = "transactionMasp2_js/transactionMasp2.wasm";
        this.zkeyPath = `transactionMasp2.zkey`;
        this.calculateWtns = require("../../build-circuits/transactionMasp2_js/witness_calculator.js");
        this.config = { in: 2, out: 2 };
    }
    parsePublicInputsFromArray(publicInputsBytes) {
        if (publicInputsBytes) {
            if (publicInputsBytes.length == 9) {
                return {
                    root: publicInputsBytes[0],
                    publicAmount: publicInputsBytes[1],
                    extDataHash: publicInputsBytes[2],
                    feeAmount: publicInputsBytes[3],
                    mintPubkey: publicInputsBytes[4],
                    nullifiers: [publicInputsBytes[5], publicInputsBytes[6]],
                    leaves: [[publicInputsBytes[7], publicInputsBytes[8]]],
                };
            }
            else {
                throw `publicInputsBytes.length invalid ${publicInputsBytes.length} != 9`;
            }
        }
        else {
            throw new Error("public input bytes undefined");
        }
    }
    getInstructions(transaction) {
        return __awaiter(this, void 0, void 0, function* () {
            if (transaction.params &&
                transaction.params.nullifierPdaPubkeys &&
                transaction.params.leavesPdaPubkeys) {
                if (!transaction.payer) {
                    throw new Error("Payer not defined");
                }
                const ix = yield this.verifierProgram.methods
                    .shieldedTransferInputs(Buffer.from(transaction.proofBytes), Buffer.from(transaction.publicInputs.publicAmount), transaction.publicInputs.nullifiers, transaction.publicInputs.leaves[0], Buffer.from(transaction.publicInputs.feeAmount), new anchor.BN(transaction.rootIndex.toString()), new anchor.BN(transaction.relayer.relayerFee.toString()), Buffer.from(transaction.encryptedUtxos.slice(0, 190)))
                    .accounts(Object.assign(Object.assign({}, transaction.params.accounts), transaction.relayer.accounts))
                    .remainingAccounts([
                    ...transaction.params.nullifierPdaPubkeys,
                    ...transaction.params.leavesPdaPubkeys,
                ])
                    .instruction();
                this.instructions = [ix];
                return [ix];
            }
            else {
                throw new Error("transaction.params, nullifierPdaPubkeys or leavesPdaPubkeys undefined");
            }
        });
    }
}
exports.VerifierZero = VerifierZero;

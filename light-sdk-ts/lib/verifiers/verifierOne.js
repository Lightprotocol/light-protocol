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
exports.VerifierOne = void 0;
const verifier_program_one_1 = require("../idls/verifier_program_one");
const anchor = __importStar(require("@coral-xyz/anchor"));
const anchor_1 = require("@coral-xyz/anchor");
const index_1 = require("../index");
class VerifierOne {
    constructor() {
        try {
            this.verifierProgram = new anchor_1.Program(verifier_program_one_1.VerifierProgramOne, index_1.verifierProgramOneProgramId);
        }
        catch (e) { }
        this.wtnsGenPath =
            "./build-circuits/transactionMasp10_js/transactionMasp10";
        this.zkeyPath = "./build-circuits/transactionMasp10";
        this.calculateWtns = require("../../build-circuits/transactionMasp10_js/witness_calculator.js");
        this.config = { in: 10, out: 2 };
    }
    parsePublicInputsFromArray(publicInputsBytes) {
        if (publicInputsBytes.length == 17) {
            return {
                root: publicInputsBytes[0],
                publicAmount: publicInputsBytes[1],
                extDataHash: publicInputsBytes[2],
                feeAmount: publicInputsBytes[3],
                mintPubkey: publicInputsBytes[4],
                nullifiers: Array.from(publicInputsBytes.slice(5, 15)),
                leaves: [[publicInputsBytes[15], publicInputsBytes[16]]],
            };
        }
        else {
            throw `publicInputsBytes.length invalid ${publicInputsBytes.length} != 17`;
        }
    }
    getInstructions(transaction) {
        return __awaiter(this, void 0, void 0, function* () {
            if (transaction.params &&
                transaction.params.nullifierPdaPubkeys &&
                transaction.params.leavesPdaPubkeys &&
                transaction.publicInputs) {
                if (!transaction.payer) {
                    throw new Error("Payer not defined");
                }
                const ix1 = yield this.verifierProgram.methods
                    .shieldedTransferFirst(Buffer.from(transaction.publicInputs.publicAmount), transaction.publicInputs.nullifiers, transaction.publicInputs.leaves[0], Buffer.from(transaction.publicInputs.feeAmount), new anchor.BN(transaction.rootIndex.toString()), new anchor.BN(transaction.relayer.relayerFee.toString()), Buffer.from(transaction.encryptedUtxos))
                    .accounts(Object.assign(Object.assign({}, transaction.params.accounts), transaction.relayer.accounts))
                    .instruction();
                const ix2 = yield this.verifierProgram.methods
                    .shieldedTransferSecond(Buffer.from(transaction.proofBytes))
                    .accounts(Object.assign(Object.assign({}, transaction.params.accounts), transaction.relayer.accounts))
                    .remainingAccounts([
                    ...transaction.params.nullifierPdaPubkeys,
                    ...transaction.params.leavesPdaPubkeys,
                ])
                    .signers([transaction.payer])
                    .instruction();
                this.instructions = [ix1, ix2];
                return this.instructions;
            }
            else {
                throw new Error("transaction.params, nullifierPdaPubkeys or leavesPdaPubkeys undefined");
            }
        });
    }
}
exports.VerifierOne = VerifierOne;

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
exports.getInsertedLeaves = exports.getUnspentUtxo = exports.getUninsertedLeaves = void 0;
const utxo_1 = require("../utxo");
const anchor = __importStar(require("@project-serum/anchor"));
const token = require('@solana/spl-token');
const web3_js_1 = require("@solana/web3.js");
function getUninsertedLeaves({ merkleTreeProgram, merkleTreeIndex, connection
// merkleTreePubkey
 }) {
    return __awaiter(this, void 0, void 0, function* () {
        var leave_accounts = yield merkleTreeProgram.account.twoLeavesBytesPda.all();
        console.log("Total nr of accounts. ", leave_accounts.length);
        let filteredLeaves = leave_accounts
            .filter((pda) => {
            return pda.account.leftLeafIndex.toNumber() >= merkleTreeIndex.toNumber();
        }).sort((a, b) => a.account.leftLeafIndex.toNumber() - b.account.leftLeafIndex.toNumber());
        return filteredLeaves.map((pda) => {
            return { isSigner: false, isWritable: false, pubkey: pda.publicKey };
        });
    });
}
exports.getUninsertedLeaves = getUninsertedLeaves;
function getUnspentUtxo(leavesPdas, provider, encryptionKeypair, KEYPAIR, FEE_ASSET, mint, POSEIDON, merkleTreeProgram) {
    return __awaiter(this, void 0, void 0, function* () {
        let decryptedUtxo1;
        for (var i = 0; i < leavesPdas.length; i++) {
            console.log("iter ", i);
            try {
                // decrypt first leaves account and build utxo
                decryptedUtxo1 = utxo_1.Utxo.decrypt(new Uint8Array(Array.from(leavesPdas[i].account.encryptedUtxos.slice(0, 71))), new Uint8Array(Array.from(leavesPdas[i].account.encryptedUtxos.slice(71, 71 + 24))), encryptionKeypair.PublicKey, encryptionKeypair, KEYPAIR, [FEE_ASSET, mint], POSEIDON, 0)[1];
                console.log("decryptedUtxo1 ", decryptedUtxo1);
                let nullifier = decryptedUtxo1.getNullifier();
                console.log("decryptedUtxo1", decryptedUtxo1);
                let nullifierPubkey = (yield web3_js_1.PublicKey.findProgramAddress([new anchor.BN(nullifier.toString()).toBuffer(), anchor.utils.bytes.utf8.encode("nf")], merkleTreeProgram.programId))[0];
                let accountInfo = yield provider.connection.getAccountInfo(nullifierPubkey);
                console.log("accountInfo ", accountInfo);
                console.log("decryptedUtxo1.amounts[1].toString()  ", decryptedUtxo1.amounts[1].toString());
                console.log("decryptedUtxo1.amounts[0].toString()  ", decryptedUtxo1.amounts[0].toString());
                if (accountInfo == null && decryptedUtxo1.amounts[1].toString() != "0" && decryptedUtxo1.amounts[0].toString() != "0") {
                    console.log("found unspent leaf");
                    return decryptedUtxo1;
                }
                else if (i == leavesPdas.length - 1) {
                    throw "no unspent leaf found";
                }
            }
            catch (error) {
                console.log(error);
            }
        }
    });
}
exports.getUnspentUtxo = getUnspentUtxo;
function getInsertedLeaves({ merkleTreeProgram, merkleTreeIndex, connection
// merkleTreePubkey
 }) {
    return __awaiter(this, void 0, void 0, function* () {
        var leave_accounts = yield merkleTreeProgram.account.twoLeavesBytesPda.all();
        console.log("Total nr of accounts. ", leave_accounts.length);
        let filteredLeaves = leave_accounts
            .filter((pda) => {
            return pda.account.leftLeafIndex.toNumber() < merkleTreeIndex.toNumber();
        }).sort((a, b) => a.account.leftLeafIndex.toNumber() - b.account.leftLeafIndex.toNumber());
        return filteredLeaves;
    });
}
exports.getInsertedLeaves = getInsertedLeaves;

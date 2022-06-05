"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const ethers_1 = require("ethers");
const tweetnacl_1 = __importDefault(require("tweetnacl"));
const keypair_1 = require("./keypair");
function deriveKeypairsFromSig(seed) {
    return __awaiter(this, void 0, void 0, function* () {
        const shieldedPrivkey = ethers_1.ethers.utils.keccak256(Buffer.from(seed));
        console.log("shieldedPrivkey", shieldedPrivkey);
        // For use into proofs
        const skp = new keypair_1.Keypair(shieldedPrivkey);
        const encryptionPrivkey = ethers_1.ethers.utils.keccak256(Buffer.from(seed));
        console.log("encryptionPrivkey", encryptionPrivkey);
        // To encrypt and decrypt and send to recipient's ekp.pubkey
        const ekp = tweetnacl_1.default.box.keyPair.fromSecretKey(Buffer.from(encryptionPrivkey.slice(2), 'hex'));
        return { skp, ekp, skpN: skp, ekpN: ekp };
    });
}
module.exports = { deriveKeypairsFromSig };

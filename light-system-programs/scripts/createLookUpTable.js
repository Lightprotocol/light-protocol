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
const zk_js_1 = require("@lightprotocol/zk.js");
const anchor = __importStar(require("@coral-xyz/anchor"));
const web3_js_1 = require("@solana/web3.js");
const fs_1 = require("fs");
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
process.env.ANCHOR_PROVIDER_URL = "https://api.testnet.solana.com";
function main() {
    return __awaiter(this, void 0, void 0, function* () {
        const privkey = JSON.parse((0, fs_1.readFileSync)(process.env.ANCHOR_WALLET, "utf8"));
        const payer = web3_js_1.Keypair.fromSecretKey(Uint8Array.from(privkey));
        // Replace this with your user's Solana wallet
        const connection = new web3_js_1.Connection(process.env.ANCHOR_PROVIDER_URL, zk_js_1.confirmConfig);
        const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(payer), zk_js_1.confirmConfig);
        let lookupTable = yield (0, zk_js_1.initLookUpTable)((0, zk_js_1.useWallet)(payer, process.env.ANCHOR_PROVIDER_URL, true, "confirmed"), provider);
        console.log("lookupTable ", lookupTable);
    });
}
main();

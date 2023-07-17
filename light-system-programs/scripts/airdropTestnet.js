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
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
process.env.ANCHOR_PROVIDER_URL = "https://api.testnet.solana.com";
const recipient = "CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG";
function main() {
    return __awaiter(this, void 0, void 0, function* () {
        console.log("airdropping 100 testnet sol to ", recipient, " in 100 transaftions");
        // Replace this with your user's Solana wallet
        const deployerSolanaWallet = zk_js_1.ADMIN_AUTH_KEYPAIR;
        const connection = new web3_js_1.Connection(process.env.ANCHOR_PROVIDER_URL, zk_js_1.confirmConfig);
        const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(deployerSolanaWallet), zk_js_1.confirmConfig);
        for (var i = 0; i < 100; i++) {
            const tmpSolanaWallet = anchor.web3.Keypair.generate();
            yield (0, zk_js_1.airdropSol)({
                provider,
                lamports: 1e9,
                recipientPublicKey: tmpSolanaWallet.publicKey,
            });
            yield (0, zk_js_1.sleep)(1000);
            const balance = yield provider.connection.getBalance(tmpSolanaWallet.publicKey);
            let tx = new web3_js_1.Transaction().add(web3_js_1.SystemProgram.transfer({
                fromPubkey: tmpSolanaWallet.publicKey,
                toPubkey: new web3_js_1.PublicKey(recipient),
                lamports: balance - 5000,
            }));
            yield (0, web3_js_1.sendAndConfirmTransaction)(provider.connection, tx, [tmpSolanaWallet], zk_js_1.confirmConfig);
            console.log(`${recipient} balance ${yield provider.connection.getBalance(new web3_js_1.PublicKey(recipient))}`);
            yield (0, zk_js_1.sleep)(10000);
        }
    });
}
main();

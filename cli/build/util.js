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
var __generator = (this && this.__generator) || function (thisArg, body) {
    var _ = { label: 0, sent: function() { if (t[0] & 1) throw t[1]; return t[1]; }, trys: [], ops: [] }, f, y, t, g;
    return g = { next: verb(0), "throw": verb(1), "return": verb(2) }, typeof Symbol === "function" && (g[Symbol.iterator] = function() { return this; }), g;
    function verb(n) { return function (v) { return step([n, v]); }; }
    function step(op) {
        if (f) throw new TypeError("Generator is already executing.");
        while (g && (g = 0, op[0] && (_ = 0)), _) try {
            if (f = 1, y && (t = op[0] & 2 ? y["return"] : op[0] ? y["throw"] || ((t = y["return"]) && t.call(y), 0) : y.next) && !(t = t.call(y, op[1])).done) return t;
            if (y = 0, t) op = [op[0] & 2, t.value];
            switch (op[0]) {
                case 0: case 1: t = op; break;
                case 4: _.label++; return { value: op[1], done: false };
                case 5: _.label++; y = op[1]; op = [0]; continue;
                case 7: op = _.ops.pop(); _.trys.pop(); continue;
                default:
                    if (!(t = _.trys, t = t.length > 0 && t[t.length - 1]) && (op[0] === 6 || op[0] === 2)) { _ = 0; continue; }
                    if (op[0] === 3 && (!t || (op[1] > t[0] && op[1] < t[3]))) { _.label = op[1]; break; }
                    if (op[0] === 6 && _.label < t[1]) { _.label = t[1]; t = op; break; }
                    if (t && _.label < t[2]) { _.label = t[2]; _.ops.push(op); break; }
                    if (t[2]) _.ops.pop();
                    _.trys.pop(); continue;
            }
            op = body.call(thisArg, _);
        } catch (e) { op = [6, e]; y = 0; } finally { f = t = 0; }
        if (op[0] & 5) throw op[1]; return { value: op[0] ? op[1] : void 0, done: true };
    }
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.readUserFromFile = exports.saveUserToFile = exports.readWalletFromFile = exports.createNewWallet = void 0;
var fs = require("fs");
var solana = require("@solana/web3.js");
// import * as light from "light-sdk";
var circomlibjs = require("circomlibjs");
var createNewWallet = function () {
    var keypair = solana.Keypair.generate();
    var secretKey = keypair.secretKey;
    try {
        fs.writeFileSync("./cache/secret.txt", JSON.stringify(Array.from(secretKey)));
        console.log("- secret created and cached");
        return keypair;
    }
    catch (e) {
        throw new Error("error writing secret.txt");
    }
};
exports.createNewWallet = createNewWallet;
var readWalletFromFile = function () {
    var secretKey = [];
    try {
        var data = fs.readFileSync("./cache/secret.txt", "utf8");
        secretKey = JSON.parse(data);
        var asUint8Array = new Uint8Array(secretKey);
        var keypair = solana.Keypair.fromSecretKey(asUint8Array);
        console.log("Wallet found!");
        return keypair;
    }
    catch (e) {
        throw new Error("secret.txt not found or corrupted!");
    }
};
exports.readWalletFromFile = readWalletFromFile;
var decryptedUtxos = [
    { test: "testString" },
    232323,
    "string",
];
var saveUserToFile = function (_a) {
    var signature = _a.signature, utxos = _a.utxos;
    fs.writeFileSync("./cache/signature.txt", JSON.stringify(signature));
    console.log("- signature cached");
    // TODO: encrypt user utxos
    fs.writeFileSync("./cache/utxos.txt", JSON.stringify(utxos));
    console.log("- utxos cached");
};
exports.saveUserToFile = saveUserToFile;
// simulates state fetching.
var readUserFromFile = function () { return __awaiter(void 0, void 0, void 0, function () {
    var signature, decryptedUtxos, data, data, signatureArray, poseidon;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0:
                decryptedUtxos = [];
                try {
                    data = fs.readFileSync("./cache/signature.txt", "utf8");
                    console.log(data);
                    signature = JSON.parse(data);
                }
                catch (e) {
                    console.log("signature.txt not found!");
                }
                try {
                    data = fs.readFileSync("./cache/utxos.txt", "utf8");
                    console.log(JSON.parse(data));
                    decryptedUtxos = JSON.parse(data);
                }
                catch (e) {
                    console.log("utxos.txt not found!");
                }
                signatureArray = Array.from(signature);
                // TODO: fetch and find user utxos (decr, encr)
                (0, exports.saveUserToFile)({ signature: signatureArray, utxos: decryptedUtxos });
                return [4 /*yield*/, circomlibjs.buildPoseidonOpt()];
            case 1:
                poseidon = _a.sent();
                // TODO: add utxos to user..., add balance etc all to user account, also keys,
                // TODO: User: add "publickey functionality"
                //   const user = new User(
                //     poseidon,
                //     new anchor.BN(signatureArray).toString("hex")
                //     wallet
                //   );
                console.log("User logged in!");
                return [2 /*return*/];
        }
    });
}); };
exports.readUserFromFile = readUserFromFile;
//# sourceMappingURL=util.js.map
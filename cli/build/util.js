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
exports.readUserFromFile = exports.saveUserToFile = exports.readWalletFromFile = exports.getConnection = exports.getAirdrop = exports.createNewWallet = void 0;
var fs = require("fs");
var solana = require("@solana/web3.js");
var light_sdk_1 = require("light-sdk");
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
function getAirdrop(wallet) {
    return __awaiter(this, void 0, void 0, function () {
        var connection, balance, amount, res, Newbalance, txTransfer1;
        return __generator(this, function (_a) {
            switch (_a.label) {
                case 0:
                    connection = (0, exports.getConnection)();
                    return [4 /*yield*/, connection.getBalance(wallet.publicKey, "confirmed")];
                case 1:
                    balance = _a.sent();
                    console.log("balance ".concat(balance, " for ").concat(wallet.publicKey.toString()));
                    if (!(balance <= 50000)) return [3 /*break*/, 6];
                    amount = 10000000000;
                    return [4 /*yield*/, connection.requestAirdrop(wallet.publicKey, amount)];
                case 2:
                    res = _a.sent();
                    return [4 /*yield*/, connection.confirmTransaction(res, "confirmed")];
                case 3:
                    _a.sent();
                    return [4 /*yield*/, connection.getBalance(wallet.publicKey)];
                case 4:
                    Newbalance = _a.sent();
                    console.assert(Newbalance == balance + amount, "airdrop failed");
                    txTransfer1 = new solana.Transaction().add(solana.SystemProgram.transfer({
                        fromPubkey: wallet.publicKey,
                        toPubkey: light_sdk_1.AUTHORITY,
                        lamports: 1000000000,
                    }));
                    return [4 /*yield*/, solana.sendAndConfirmTransaction(connection, txTransfer1, [wallet], light_sdk_1.confirmConfig)];
                case 5:
                    _a.sent();
                    return [3 /*break*/, 7];
                case 6:
                    console.log("no airdrop needed");
                    _a.label = 7;
                case 7: return [2 /*return*/];
            }
        });
    });
}
exports.getAirdrop = getAirdrop;
var getConnection = function () {
    return new solana.Connection("http://127.0.0.1:8899");
};
exports.getConnection = getConnection;
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
var saveUserToFile = function (_a) {
    var user = _a.user;
    /**
     * This represents the UIs state. (not localstorage!)
     * This should just store the whole user object.
     * TODO: store whole object (fix JSON serialization)
     * */
    var userToCache = {
        //@ts-ignore
        seed: user.seed,
        payerSecret: Array.from(user.payer.secretKey),
        utxos: user.utxos,
    };
    fs.writeFileSync("./cache/user.txt", JSON.stringify(userToCache));
    console.log("- user cached");
};
exports.saveUserToFile = saveUserToFile;
// simulates state fetching.
var readUserFromFile = function () { return __awaiter(void 0, void 0, void 0, function () {
    var cachedUser, data, asUint8Array, rebuiltUser, lightInstance, user, e_1;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0:
                try {
                    data = fs.readFileSync("./cache/user.txt", "utf8");
                    cachedUser = JSON.parse(data);
                }
                catch (e) {
                    console.log("user.txt snot found!");
                }
                asUint8Array = new Uint8Array(cachedUser.payerSecret);
                rebuiltUser = {
                    seed: cachedUser.seed,
                    payer: solana.Keypair.fromSecretKey(asUint8Array),
                    utxos: cachedUser.utxos,
                };
                _a.label = 1;
            case 1:
                _a.trys.push([1, 4, , 5]);
                return [4 /*yield*/, (0, light_sdk_1.getLightInstance)()];
            case 2:
                lightInstance = _a.sent();
                user = new light_sdk_1.User({ payer: rebuiltUser.payer, lightInstance: lightInstance });
                console.log("loading user...");
                //@ts-ignore
                return [4 /*yield*/, user.load(rebuiltUser)];
            case 3:
                //@ts-ignore
                _a.sent();
                console.log("✔️ User built from state!");
                return [2 /*return*/, user];
            case 4:
                e_1 = _a.sent();
                console.log("err:", e_1);
                return [3 /*break*/, 5];
            case 5: return [2 /*return*/];
        }
    });
}); };
exports.readUserFromFile = readUserFromFile;
//# sourceMappingURL=util.js.map
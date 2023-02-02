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
exports.handler = exports.builder = exports.desc = exports.command = void 0;
var util_1 = require("../util");
var anchor = require("@coral-xyz/anchor");
var light_sdk_1 = require("light-sdk");
exports.command = "transfer";
exports.desc = "create, send and confirm an transfer transaction for given, <amount> <token>, and to <recipient>";
var builder = function (yargs) {
    return yargs.options({
        amount: { type: "number" },
        token: { type: "string" },
        shieldedRecipient: { type: "string" },
        encryptionPublicKey: { type: "string" },
    });
};
exports.builder = builder;
var handler = function (argv) { return __awaiter(void 0, void 0, void 0, function () {
    var amount, token, shieldedRecipient, encryptionPublicKey, user, e_1, recipient, recipientEncryptionPublicKey;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0:
                process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
                process.env.ANCHOR_WALLET = "./cache/secret.txt";
                amount = argv.amount, token = argv.token, shieldedRecipient = argv.shieldedRecipient, encryptionPublicKey = argv.encryptionPublicKey;
                _a.label = 1;
            case 1:
                _a.trys.push([1, 3, , 4]);
                return [4 /*yield*/, (0, util_1.readUserFromFile)()];
            case 2:
                user = _a.sent();
                return [3 /*break*/, 4];
            case 3:
                e_1 = _a.sent();
                throw new Error("No user.txt file found, please login first.");
            case 4:
                recipient = new anchor.BN(shieldedRecipient, "hex");
                recipientEncryptionPublicKey = (0, light_sdk_1.strToArr)(encryptionPublicKey);
                return [4 /*yield*/, user.transfer({
                        amount: amount * 1e9,
                        token: token,
                        recipient: recipient,
                        recipientEncryptionPublicKey: recipientEncryptionPublicKey,
                    })];
            case 5:
                _a.sent();
                console.log("Shielded Transfer done: ".concat(amount, " ").concat(token));
                process.exit(0);
                return [2 /*return*/];
        }
    });
}); };
exports.handler = handler;
//# sourceMappingURL=transfer.js.map
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
var __spreadArray = (this && this.__spreadArray) || function (to, from, pack) {
    if (pack || arguments.length === 2) for (var i = 0, l = from.length, ar; i < l; i++) {
        if (ar || !(i in from)) {
            if (!ar) ar = Array.prototype.slice.call(from, 0, i);
            ar[i] = from[i];
        }
    }
    return to.concat(ar || Array.prototype.slice.call(from));
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.Wallet = void 0;
/// TODO: extract wallet into its own npm package
var web3_js_1 = require("@solana/web3.js");
var tweetnacl_1 = require("tweetnacl");
var psp_compressed_pda_1 = require("../idls/psp_compressed_pda");
/// Mock Solana web3 library
var Wallet = /** @class */ (function () {
    function Wallet(keypair, url, commitment) {
        var _this = this;
        this.signTransaction = function (tx) { return __awaiter(_this, void 0, void 0, function () {
            return __generator(this, function (_a) {
                switch (_a.label) {
                    case 0: return [4 /*yield*/, tx.sign([this._keypair])];
                    case 1:
                        _a.sent();
                        return [2 /*return*/, tx];
                }
            });
        }); };
        this.sendTransaction = function (transaction) { return __awaiter(_this, void 0, void 0, function () {
            var signature;
            return __generator(this, function (_a) {
                switch (_a.label) {
                    case 0: return [4 /*yield*/, this._connection.sendTransaction(transaction)];
                    case 1:
                        signature = _a.sent();
                        return [2 /*return*/, signature];
                }
            });
        }); };
        this.signAllTransactions = function (transactions) { return __awaiter(_this, void 0, void 0, function () {
            var signedTxs;
            var _this = this;
            return __generator(this, function (_a) {
                switch (_a.label) {
                    case 0: return [4 /*yield*/, Promise.all(transactions.map(function (tx) { return __awaiter(_this, void 0, void 0, function () {
                            return __generator(this, function (_a) {
                                switch (_a.label) {
                                    case 0: return [4 /*yield*/, this.signTransaction(tx)];
                                    case 1: return [2 /*return*/, _a.sent()];
                                }
                            });
                        }); }))];
                    case 1:
                        signedTxs = _a.sent();
                        return [2 /*return*/, signedTxs];
                }
            });
        }); };
        this.signMessage = function (message) { return __awaiter(_this, void 0, void 0, function () {
            return __generator(this, function (_a) {
                return [2 /*return*/, tweetnacl_1.sign.detached(message, this._keypair.secretKey)];
            });
        }); };
        this.sendAndConfirmTransaction = function (transaction, signers) {
            if (signers === void 0) { signers = []; }
            return __awaiter(_this, void 0, void 0, function () {
                var response;
                return __generator(this, function (_a) {
                    switch (_a.label) {
                        case 0: return [4 /*yield*/, (0, web3_js_1.sendAndConfirmTransaction)(this._connection, transaction, __spreadArray([this._keypair], signers, true), {
                                commitment: this._commitment,
                            })];
                        case 1:
                            response = _a.sent();
                            return [2 /*return*/, response];
                    }
                });
            });
        };
        this.getProof = function (proofInputs) { return __awaiter(_this, void 0, void 0, function () {
            var _verifierIdl, _circuitName;
            return __generator(this, function (_a) {
                _verifierIdl = getIdlByProofInputs(proofInputs);
                _circuitName = getCircuitByProofInputs(proofInputs);
                // const { parsedProof, parsedPublicInputsObject } = await getProofInternal({
                //   /// TODO: implement actual path
                //   firstPath: "mockPath",
                //   verifierIdl,
                //   circuitName,
                //   proofInputs,
                //   enableLogging: true,
                //   verify: true,
                // });
                return [2 /*return*/, {
                        parsedProof: "mockParsedProof",
                        parsedPublicInputsObject: "mockParsedPublicInputsObject",
                    }];
            });
        }); };
        this._publicKey = keypair.publicKey;
        this._keypair = keypair;
        this._connection = new web3_js_1.Connection(url);
        this._url = url;
        this._commitment = commitment;
    }
    return Wallet;
}());
exports.Wallet = Wallet;
/// TODO: generalize when needed
var getIdlByProofInputs = function (_proofInputs) {
    return psp_compressed_pda_1.IDL;
};
/// TODO: use actual circuits
/// Picks the circuit by amount of proof inputs
var getCircuitByProofInputs = function (_proofInputs) {
    return "mockCircuit";
};

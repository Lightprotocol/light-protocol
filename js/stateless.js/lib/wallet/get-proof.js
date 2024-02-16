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
exports.getProofInternal = void 0;
var prover_js_1 = require("@lightprotocol/prover.js");
var errors_1 = require("../errors");
function getProverInstance(verifierIdl, firstPath, circuitName) {
    return new prover_js_1.Prover(verifierIdl, firstPath, circuitName);
}
/// The tx is not yet created at the time of proof generation. Therefore,
/// we need to call the getProof before requesting a signature. That's ok
/// because proof generation in the wallet doesn't require a secret!
function getProofInternal(_a) {
    var proofInputs = _a.proofInputs, verifierIdl = _a.verifierIdl, firstPath = _a.firstPath, circuitName = _a.circuitName, _b = _a.getProver, getProver = _b === void 0 ? getProverInstance : _b, _c = _a.verify, verify = _c === void 0 ? true : _c, enableLogging = _a.enableLogging, wasmTester = _a.wasmTester;
    return __awaiter(this, void 0, void 0, function () {
        var prover, prefix, logMsg, parsedProof, parsedPublicInputs, result, error_1, res, parsedPublicInputsObject;
        return __generator(this, function (_d) {
            switch (_d.label) {
                case 0: return [4 /*yield*/, getProver(verifierIdl, firstPath, circuitName, wasmTester)];
                case 1:
                    prover = _d.sent();
                    return [4 /*yield*/, prover.addProofInputs(proofInputs)];
                case 2:
                    _d.sent();
                    prefix = "\u001B[37m[".concat(new Date(Date.now()).toISOString(), "]\u001B[0m");
                    logMsg = "".concat(prefix, " Proving ").concat(verifierIdl.name, " circuit");
                    if (enableLogging)
                        console.time(logMsg);
                    _d.label = 3;
                case 3:
                    _d.trys.push([3, 5, , 6]);
                    return [4 /*yield*/, prover.fullProveAndParse()];
                case 4:
                    result = _d.sent();
                    parsedProof = result.parsedProof;
                    parsedPublicInputs = result.parsedPublicInputs;
                    return [3 /*break*/, 6];
                case 5:
                    error_1 = _d.sent();
                    throw new errors_1.ProofError(errors_1.ProofErrorCode.PROOF_GENERATION_FAILED, "getProofInternal", error_1);
                case 6:
                    /// debug
                    if (enableLogging)
                        console.timeEnd(logMsg);
                    if (!(verify || enableLogging)) return [3 /*break*/, 8];
                    return [4 /*yield*/, prover.verify()];
                case 7:
                    res = _d.sent();
                    if (!res)
                        throw new errors_1.ProofError(errors_1.ProofErrorCode.INVALID_PROOF, "getProofInternal");
                    _d.label = 8;
                case 8:
                    parsedPublicInputsObject = prover.parsePublicInputsFromArray(parsedPublicInputs);
                    return [2 /*return*/, { parsedProof: parsedProof, parsedPublicInputsObject: parsedPublicInputsObject }];
            }
        });
    });
}
exports.getProofInternal = getProofInternal;

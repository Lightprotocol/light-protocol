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
var __exportStar = (this && this.__exportStar) || function(m, exports) {
    for (var p in m) if (p !== "default" && !Object.prototype.hasOwnProperty.call(exports, p)) __createBinding(exports, m, p);
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.generateRandomTestAmount = void 0;
__exportStar(require("./createAccounts"), exports);
__exportStar(require("./testChecks"), exports);
__exportStar(require("./setUpMerkleTree"), exports);
__exportStar(require("./initLookUpTable"), exports);
__exportStar(require("./constants_market_place"), exports);
__exportStar(require("./functionalCircuit"), exports);
__exportStar(require("./constants_system_verifier"), exports);
__exportStar(require("./updateMerkleTree"), exports);
__exportStar(require("./testRelayer"), exports);
__exportStar(require("./userTestAssertHelper"), exports);
__exportStar(require("./testTransaction"), exports);
__exportStar(require("./airdrop"), exports);
function generateRandomTestAmount(min = 0.2, max = 2, decimals) {
    const randomAmount = Math.random() * (max - min) + min;
    return +randomAmount.toFixed(decimals);
}
exports.generateRandomTestAmount = generateRandomTestAmount;
//# sourceMappingURL=index.js.map
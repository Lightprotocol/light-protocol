"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.generateRandomTestAmount = void 0;
const tslib_1 = require("tslib");
tslib_1.__exportStar(require("./createAccounts"), exports);
tslib_1.__exportStar(require("./testChecks"), exports);
tslib_1.__exportStar(require("./setUpMerkleTree"), exports);
tslib_1.__exportStar(require("./initLookUpTable"), exports);
tslib_1.__exportStar(require("./constants_market_place"), exports);
tslib_1.__exportStar(require("./functionalCircuit"), exports);
tslib_1.__exportStar(require("./constants_system_verifier"), exports);
tslib_1.__exportStar(require("./updateMerkleTree"), exports);
tslib_1.__exportStar(require("./testRelayer"), exports);
tslib_1.__exportStar(require("./userTestAssertHelper"), exports);
tslib_1.__exportStar(require("./testTransaction"), exports);
tslib_1.__exportStar(require("./airdrop"), exports);
function generateRandomTestAmount(min = 0.2, max = 2, decimals) {
    const randomAmount = Math.random() * (max - min) + min;
    return +randomAmount.toFixed(decimals);
}
exports.generateRandomTestAmount = generateRandomTestAmount;
//# sourceMappingURL=index.js.map
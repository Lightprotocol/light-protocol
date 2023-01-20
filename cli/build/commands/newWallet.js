"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.handler = exports.builder = exports.desc = exports.command = void 0;
var util_1 = require("../util");
exports.command = "new wallet";
exports.desc = "Generate a new Solana wallet (secret key)";
var builder = function (yargs) { return yargs; };
exports.builder = builder;
var handler = function () {
    (0, util_1.createNewWallet)();
    (0, util_1.readWalletFromFile)();
    process.exit(0);
};
exports.handler = handler;
//# sourceMappingURL=newWallet.js.map
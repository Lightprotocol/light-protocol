"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.prepareTransaction = void 0;
const ethers_1 = require("ethers");
const enums_1 = require("./enums");
const utxos_1 = __importDefault(require("./utxos"));
const prepareTransaction = (inputUtxos = [], outputUtxos = [], relayFee, action) => {
    /// Validation
    if (inputUtxos.length > 16 || outputUtxos.length > 2) {
        throw new Error('Incorrect inputUtxos/outputUtxos count');
    }
    /// fill inputUtxos until 2 or 16
    while (inputUtxos.length !== 2 && inputUtxos.length < 16) {
        inputUtxos.push(new utxos_1.default());
    }
    /// if there are no outputUtxo add one
    while (outputUtxos.length < 2) {
        outputUtxos.push(new utxos_1.default());
    }
    /// the fee plus the amount to pay has to be bigger than the amount in the input utxo
    // which doesn't make sense it should be the other way arround right
    const externalAmountBigNumber = ethers_1.BigNumber.from(relayFee.toString())
        .add(outputUtxos.reduce((sum, utxo) => sum.add(utxo.amount), ethers_1.BigNumber.from(0)))
        .sub(inputUtxos.reduce((sum, utxo) => sum.add(utxo.amount), ethers_1.BigNumber.from(0)));
    /// if it is a deposit and the amount going in is smaller than 0 throw error
    if (enums_1.Action[action] === 'deposit' &&
        Number(externalAmountBigNumber.toString()) < 0) {
        throw new Error(`Incorrect Extamount: ${Number(externalAmountBigNumber.toString())}`);
    }
    return {
        inputUtxos,
        outputUtxos,
        externalAmountBigNumber,
    };
};
exports.prepareTransaction = prepareTransaction;

"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.createOutputUtxo = void 0;
const ethers_1 = require("ethers");
const utxos_1 = __importDefault(require("./utxos"));
const createOutputUtxo = function (inputUtxos, amount, shieldedKeypair, relayerFee) {
    // TODO here is a problem regarding BIGNUMBER / NUMBER type for amount
    // WHAT IS GOING ON
    // If its a number there is no function add / sub
    // if it is BigNumber code fails at other places ( for example line 61 in utxos.ts getNullifier)
    let inUtxoAmount = inputUtxos[0].amount;
    console.log(`inUtxo -> ${Number(inUtxoAmount)}`);
    // TODO i put this check to transform amount into Bignumber so we can use .add and .sub check if this works
    // if( typeof inUtxoAmount === "number"){
    //     inUtxoAmount = BigNumber.from(inUtxoAmount.toString());
    // }
    // This is never called i think
    if (inputUtxos.length > 1) {
        inUtxoAmount = inUtxoAmount.add(inputUtxos[1].amount);
    }
    let sendAmount = Number(amount);
    console.log(`sendAmount -> ${sendAmount}`);
    const changeUtxoAmount = inUtxoAmount
        .sub(ethers_1.BigNumber.from(sendAmount.toString()))
        .sub(ethers_1.BigNumber.from(relayerFee.toString()));
    console.log(`changeUtxo -> ${Number(changeUtxoAmount)}`);
    let changeUtxo = new utxos_1.default(changeUtxoAmount, shieldedKeypair);
    return changeUtxo;
};
exports.createOutputUtxo = createOutputUtxo;

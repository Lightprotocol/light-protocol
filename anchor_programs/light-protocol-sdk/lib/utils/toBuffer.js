"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.toBuffer = void 0;
const ethers_1 = require("ethers");
const toBuffer = (value, length) => Buffer.from(ethers_1.BigNumber.from(value)
    .toHexString()
    .slice(2)
    .padStart(length * 2, '0'), 'hex');
exports.toBuffer = toBuffer;

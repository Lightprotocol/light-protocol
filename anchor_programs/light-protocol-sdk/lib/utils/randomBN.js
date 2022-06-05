"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.randomBN = void 0;
const ethers_1 = require("ethers");
const crypto = require('crypto');
const randomBN = (nbytes = 31) => ethers_1.BigNumber.from(crypto.randomBytes(nbytes));
exports.randomBN = randomBN;

"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.randomBN = void 0;
// const ethers_1 = require("ethers");
const anchor = require("@project-serum/anchor")

const crypto = require('crypto');
const randomBN = (nbytes = 31) => new anchor.BN(crypto.randomBytes(nbytes));
exports.randomBN = randomBN;

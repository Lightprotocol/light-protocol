"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.poseidonHash2 = exports.poseidonHash = void 0;
const ethers_1 = require("ethers");
const { poseidon } = require('circomlib');
const poseidonHash = (items) => ethers_1.BigNumber.from(poseidon(items).toString());
exports.poseidonHash = poseidonHash;
const poseidonHash2 = (a, b) => (0, exports.poseidonHash)([a, b]);
exports.poseidonHash2 = poseidonHash2;

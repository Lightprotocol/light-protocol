"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.poseidonHash2 = exports.poseidonHash = void 0;
const ethers_1 = require("ethers");
const circomlibjs = require('circomlibjs');
const poseidonHash = async (items) => {
  let poseidon = await circomlibjs.buildPoseidonOpt();
  ethers_1.BigNumber.from(poseidon(items))
};
exports.poseidonHash = poseidonHash;
const poseidonHash2 = (a, b) => (0, exports.poseidonHash)([a, b]);
exports.poseidonHash2 = poseidonHash2;

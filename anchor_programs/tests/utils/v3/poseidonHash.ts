const ethers_1 = require("ethers");
const circomlibjs = require('circomlibjs');
export const poseidonHash = async (items) => {
  let poseidon = await circomlibjs.buildPoseidonOpt();
  ethers_1.BigNumber.from(poseidon(items))
};
export const poseidonHash2 = (a, b) => (0, exports.poseidonHash)([a, b]);

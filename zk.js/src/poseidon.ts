/// load once globally.
const circomlibjs = require("circomlibjs");

const poseidonPromise = circomlibjs.buildPoseidonOpt();

export function getPoseidon() {
  return poseidonPromise;
}

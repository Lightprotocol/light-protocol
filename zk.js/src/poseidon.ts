/// load once globally.
const circomlibjs = require("circomlibjs");

let poseidonPromise = circomlibjs.buildPoseidonOpt();

export function getPoseidon() {
  return poseidonPromise;
}

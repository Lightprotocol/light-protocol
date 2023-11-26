/// load once globally.
import circomlibjs from "circomlibjs";

let poseidonPromise = circomlibjs.buildPoseidonOpt();

export function getPoseidon() {
  return poseidonPromise;
}

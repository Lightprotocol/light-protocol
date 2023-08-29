pragma circom 2.0.0;

include "../../node_modules/circomlib/circuits/poseidon.circom";

template ProverTest() {
    signal input x;
    signal input hash;

    component poseidon = Poseidon(1);
    poseidon.inputs[0] <== x;
    hash === poseidon.out;
}
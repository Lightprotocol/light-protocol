pragma circom 2.1.4;
include "poseidon.circom";

template {{circom-name}}() {
    signal input x;
    signal input hash;

    component poseidon = Poseidon(1);
    poseidon.inputs[0] <== x;
    hash === poseidon.out;
}
pragma circom 2.0.0;

/*This circuit template checks that c is the multiplication of a and b.*/
include "../../node_modules/circomlib/circuits/poseidon.circom";

template Multiplier2 () {

   // Declaration of signals.
   signal input a;
   signal input b;
   signal output c;

   // Constraints.
   c <== a * b;
}

template Hash () {
    signal input a;
    signal input b;
    signal input c;
    component hasher = Poseidon(2);
    hasher.inputs[0] <== a;
    hasher.inputs[1] <== b;
    log(hasher.out);
    c === hasher.out;

}

// component main = Multiplier2();
component main {public [c]} = Hash();

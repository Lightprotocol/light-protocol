# Description

prover.js offers swift access to circom circuits' inputs that makes implementing zk cryptography with [Circom](https://github.com/iden3/circom) and [SnarkJS](https://github.com/iden3/snarkjs) more straight-forward:

- No need to navigate to the circuit code to read input name or types.
- No need to debug or track input errors from the circom compiler.
- Precompile errors pop up indicating input name or type non-compliance.

## Install

`yarn install` to install the dependencies.

## Test

`yarn test` to run prover class test.

## Example

```typescript
import { verifierIdl } from "../project_name/target/types/project_name.ts";
import { ProofInputs, ParsedPublicInputs } from "generics";

let proofInputs: ProofInputs<typeof verifierIdl, "circuitName">;

// click "ctrl + space" after the . to open a small window with all inputs and their assigned types
// any input name or type non-compliance will provoke a precompile error
proofInputs.proofInputs = {
  input1: ["12343", "343563"],
  input2: "13415144135145",
  inputn: [
    ["1", "0"],
    ["0", "1"],
  ],
};
```

## Notes

- The generic types for a circuit proof or public inputs are accessible as long as ZK account structs are available in the [Anchor](https://www.anchor-lang.com/docs/cli#idl) idl.
- The circom circuit inputs should be written in camel case for compliance with the ZK account struct rust parser.
- The generics file should contain a union type of all the verifierIdls containing the parsed ZK accounts.

## Remarks

- The generic prover class functions seamlessly even in the absence of generic type inputs (tested!).
- Making the prover class generic has no notable effect on the behaviour of the class.
- The use of circuit generic types is efficient when the inputs object is written manually and assigned to the generic type as shown in the example.

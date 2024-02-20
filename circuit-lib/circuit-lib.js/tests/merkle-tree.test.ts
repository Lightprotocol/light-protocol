import { MerkleTree } from "./utils/merkle-tree";
import { readFileSync, writeFileSync } from "fs";

const snarkjs = require("snarkjs");

import { LightWasm, WasmFactory } from "@lightprotocol/account.rs";

describe("Tests", () => {
  let lightWasm: LightWasm;

  function zk(tree_height: number, num_utxos: number): String {
    return `../circuitlib-rs/test-data/merkle${tree_height}_${num_utxos}/circuit.zkey`;
  }

  function wasm(tree_height: number, num_utxos: number): Buffer {
    let path = `../circuitlib-rs/test-data/merkle${tree_height}_${num_utxos}/circuit.wasm`;
    return readFileSync(path);
  }

  function witnessGenerator(tree_height: number, num_utxos: number): any {
    const path = "./utils/witness_calculator.js";
    const wtns = require(path);
    return wtns;
  }

  before(async () => {
    lightWasm = await WasmFactory.getInstance();
  });

  it("merkle proofgen", async () => {
    const hasher = await WasmFactory.getInstance();
    const merkleHeights = [22]; //[22, 30, 40, 128];
    const utxos = [1, 2, 3, 4, 8];
    const outPath = "/tmp";
    for (let i = 0; i < merkleHeights.length; i++) {
      for (let j = 0; j < utxos.length; j++) {
        const completePathZkey = zk(merkleHeights[i], utxos[j]);
        const buffer = wasm(merkleHeights[i], utxos[j]);
        // const leaf = "1"; //hasher.poseidonHashString(["1"]);
        const leaf = hasher.poseidonHashString(["1"]);
        const merkleTree = new MerkleTree(merkleHeights[i], hasher, [leaf]);

        let inputs = {
          root: new Array(utxos[j]).fill(merkleTree.root()),
          inPathIndices: new Array(utxos[j]).fill(merkleTree.indexOf(leaf)),
          inPathElements: new Array(utxos[j]).fill(
            merkleTree.path(merkleTree.indexOf(leaf)).pathElements,
          ),
          leaf: new Array(utxos[j]).fill(leaf),
        };

        const inputs_json = JSON.stringify(inputs);
        writeFileSync(
          `${outPath}/inputs${merkleHeights[i]}_${utxos[j]}.json`,
          inputs_json,
        );

        let generator = witnessGenerator(merkleHeights[i], utxos[j]);
        let witnessCalculator = await generator(buffer);

        console.time(`Witness generation for ${merkleHeights[i]} ${utxos[j]}`);
        let wtns = await witnessCalculator.calculateWTNSBin(inputs, 0);
        console.timeEnd(
          `Witness generation for ${merkleHeights[i]} ${utxos[j]}`,
        );

        console.time(`Proof generation for ${merkleHeights[i]} ${utxos[j]}`);
        const { proof, publicSignals } = await snarkjs.groth16.prove(
          completePathZkey,
          wtns,
        );
        console.timeEnd(`Proof generation for ${merkleHeights[i]} ${utxos[j]}`);

        // write publicSignals to json file
        const json = JSON.stringify(publicSignals);
        writeFileSync(
          `${outPath}/public_inputs_merkle${merkleHeights[i]}_${utxos[j]}.json`,
          json,
        );

        const vKey = await snarkjs.zKey.exportVerificationKey(completePathZkey);
        const res = await snarkjs.groth16.verify(vKey, publicSignals, proof);
        if (res === true) {
          console.log("Verification OK");
        } else {
          console.log("Invalid proof");
          throw new Error("Invalid Proof");
        }
      }
    }
  });
});

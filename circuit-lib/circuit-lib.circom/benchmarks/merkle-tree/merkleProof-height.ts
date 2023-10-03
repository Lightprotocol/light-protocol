const bench = require("micro-bmark");
const buildPoseidonOpt = require("circomlibjs").buildPoseidonOpt;
const wasm_tester = require("circom_tester").wasm;
const assert = require("assert");

import { genRandomSalt as generateRandomFieldElement } from "maci-crypto";
import { Provider as LightProvider } from "@lightprotocol/zk.js";
import { MerkleTree } from "../../../circuit-lib.js/src";

function getSignalByName(
  circuit: any,
  witness: any,
  signalName: string,
): string {
  const signal = `main.${signalName}`;
  return witness[circuit.symbols[signal].varIdx].toString();
}

function generateCircuitInputs(merkleTree: MerkleTree): {
  leaf: string;
  pathElements: string[];
  pathIndices: string;
} {
  const leaf = merkleTree.elements()[2];
  const pathElements = merkleTree.path(merkleTree.indexOf(leaf)).pathElements;
  const pathIndices = merkleTree.indexOf(leaf).toString();
  const circuitInputs = {
    leaf,
    pathElements,
    pathIndices,
  };

  return circuitInputs;
}

async function loadMerkleTree(
  height: number,
  hash: any,
  elements: string[],
): Promise<MerkleTree> {
  const lightProvider = await LightProvider.loadMock();
  lightProvider.solMerkleTree!.merkleTree = new MerkleTree(
    height,
    hash,
    elements,
  );
  return lightProvider.solMerkleTree?.merkleTree!;
}

async function markMerkleTree(merkleTree: MerkleTree, circuit: any) {
  const circuitInputs = generateCircuitInputs(merkleTree);
  const witness = await circuit.calculateWitness(circuitInputs);
  await circuit.checkConstraints(witness);
  await circuit.loadSymbols();

  const root = getSignalByName(circuit, witness, "root");
  assert(root, merkleTree.root());

  return root;
}

const { compare, run } = bench;
run(async () => {
  let poseidon = await buildPoseidonOpt();
  let elements = new Array<string>();
  for (let i = 0; i < 2 ** 16; i++) {
    elements.push(generateRandomFieldElement().toString());
  }

  let circuitPromises = new Array<Promise<any>>();
  let merkleTreePromises = new Array<Promise<MerkleTree>>();

  [18, 20, 22, 24, 26].map(async (height) => {
    circuitPromises.push(
      wasm_tester(`./tests/merkle-tree/merkleProof_test${height}.circom`, {
        include: "node_modules/circomlib/circuits",
      }),
    );
    merkleTreePromises.push(loadMerkleTree(Number(height), poseidon, elements));
  });
  const circuits = await Promise.all(circuitPromises);
  const merkleTrees = await Promise.all(merkleTreePromises);

  await compare("\x1b[35mBenchmarking MerkleProof Circuit\x1b[0m", 2000, {
    height_18: async () => markMerkleTree(merkleTrees[0], circuits[0]),
    height_20: async () => markMerkleTree(merkleTrees[1], circuits[1]),
    height_22: async () => markMerkleTree(merkleTrees[2], circuits[2]),
    height_24: async () => markMerkleTree(merkleTrees[3], circuits[3]),
    height_26: async () => markMerkleTree(merkleTrees[4], circuits[4]),
  });
  bench.utils.logMem(); // Log current RAM
});

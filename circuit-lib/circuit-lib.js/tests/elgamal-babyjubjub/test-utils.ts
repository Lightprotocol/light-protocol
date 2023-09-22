import { encode } from "../../src/elgamal-babyjubjub";
import * as crypto from "crypto";
import { ExtPointType } from "@noble/curves/abstract/edwards";

/**
 * Utility function used to get a random bigint.
 */
function getRandomBytes(byte_number: number): bigint {
  return BigInt("0x" + crypto.randomBytes(byte_number).toString("hex"));
}

/**
 * Utility function to generate a random 32-bit bigint encoded as a point on the Baby Jubjub curve
 */
function generateRandomEncodedMessage(): ExtPointType {
  return encode(getRandomBytes(4));
}

/**
 * - Returns a signal value similar to the "callGetSignalByName" function from the "circom-helper" package.
 * - This function depends on the "circom_tester" package.
 *
 * Example usage:
 *
 * ```typescript
 * const wasm_tester = require('circom_tester').wasm;
 *
 * /// the circuit is loaded only once and it is available for use across multiple test cases.
 * const circuit = await wasm_tester(path.resolve("./circuit/path"));
 * const witness = await circuit.calculateWitness(inputsObject);
 * await circuit.checkConstraints(witness);
 * await circuit.loadSymbols();
 *
 * /// You can check signal names by printing "circuit.symbols".
 * /// You will mostly need circuit inputs and outputs.
 * const singalName = 'main.out'
 * const signalValue = getSignalByName(circuit, witness, SignalName)
 * ```
 */
function getSignalByName(circuit: any, witness: any, signalName: string) {
  const signal = `main.${signalName}`;
  return witness[circuit.symbols[signal].varIdx].toString();
}

export { getSignalByName, getRandomBytes, generateRandomEncodedMessage };

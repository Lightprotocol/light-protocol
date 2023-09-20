import { executeAnchor, executeMacroCircom } from "./toolchain";
import { extractFilename, findFile } from "./utils";
import { generateCircuit } from "./buildCircom";
import { Flags } from "@oclif/core";
import { isCamelCase } from "@lightprotocol/zk.js";

const suffix = "Main.circom";

/**
 * Builds a Private Solana Program (PSP) given a circuit directory.
 * Creates circom files, builds the circom circuit, and compiles the anchor program.
 * @param circuitDir - The directory containing the circuit files.
 * @returns {Promise<void>}
 */
export async function buildPSP({
  circuitDir,
  ptau,
  programName,
  skipAnchor,
  skipCircuit,
  skipMacroCircom,
  circuitName,
  linkedCircuitLibraries = [],
  skipLinkCircuitlib,
  skipLinkCircomlib,
}: {
  circuitDir: string;
  ptau: number;
  programName: string;
  skipAnchor?: boolean;
  skipCircuit?: boolean;
  skipMacroCircom?: boolean;
  circuitName?: string[];
  linkedCircuitLibraries?: string[];
  skipLinkCircuitlib?: boolean;
  skipLinkCircomlib?: boolean;
}) {
  // TODO: add support to compile only selected circuits
  let foundCircuitNames: string[] = [];
  if (!skipCircuit) {
    if (!skipMacroCircom) {
      let circuits = findFile({
        directory: circuitDir,
        extension: ".light",
      });
      console.log("circuits ", circuits);
      for (let { filename, fullPath } of circuits) {
        console.log("📜 Generating circom files");
        let stdout = await executeMacroCircom({
          args: [fullPath, programName],
        });
        console.log("✅ Circom files generated successfully");
        const circuitMainFileName = extractFilename(stdout.toString().trim());
        console.log("🛠️️  Building circuit", circuitMainFileName);
        if (!circuitMainFileName)
          throw new Error("Could not extract circuit main file name");
        // not necessary because we are finding all Main.circom files later
        foundCircuitNames.push(circuitMainFileName.slice(0, -suffix.length));
      }
    }
    let circuits = findFile({
      directory: circuitDir,
      extension: "Main.circom",
    });
    for (let { filename, fullPath, light } of circuits) {
      // skip main files from macro circom generated main circom files
      if (light) continue;
      foundCircuitNames.push(filename.slice(0, -suffix.length));
    }
  }
  foundCircuitNames = [...new Set(foundCircuitNames)];
  console.log("foundCircuitNames ", foundCircuitNames);

  if (!skipLinkCircomlib) {
    linkedCircuitLibraries.push("node_modules/circomlib/circuits/");
  }
  if (!skipLinkCircuitlib) {
    linkedCircuitLibraries.push(
      `node_modules/@lightprotocol/circuit-lib.circom/src/light-utils`
    );
    linkedCircuitLibraries.push(
      `node_modules/@lightprotocol/circuit-lib.circom/src/merkle-tree`
    );
  }
  // TODO: enable multiple programs
  // TODO: add add-psp command which adds a second psp
  // TODO: add add-circom-circuit command which inits a new circom circuit of name circuitName
  // TODO: add add-circuit command which inits a new .light file of name circuitName
  if (foundCircuitNames.length > 0) {
    for (let foundCircuitName of foundCircuitNames) {
      // if circuitName is provided skip circuits which have not been provided in the circuitName flag
      if (circuitName && circuitName.indexOf(foundCircuitName) === -1) continue;

      console.log("🔑 Generating circuit ", foundCircuitName);
      await generateCircuit({
        circuitName: foundCircuitName,
        ptau,
        programName,
        linkedCircuitLibraries,
      });
      console.log(`✅ Circuit ${foundCircuitName} generated successfully`);
    }
  } else {
    throw new Error("No circuit found");
  }
  if (skipAnchor) return;
  console.log("🛠  Building on-chain program");
  await executeAnchor({ args: ["build"] });
  console.log("✅ Build finished successfully");
}

export const buildFlags = {
  name: Flags.string({ description: "Name of the project." }),
  ptau: Flags.integer({ description: "Ptau value.", default: 15 }),
  circuitDir: Flags.string({
    description: "Directory of the circuit.",
    default: "circuits",
  }),
  skipAnchor: Flags.boolean({
    description: "Directory of the circuit.",
    default: false,
    required: false,
  }),
  skipCircuit: Flags.boolean({
    description: "Directory of the circuit.",
    default: false,
    required: false,
  }),
  skipMacroCircom: Flags.boolean({
    description: "Directory of the circuit.",
    default: false,
    required: false,
  }),
  circuitName: Flags.string({
    description:
      "Name of circuit main file, the name has to be camel case and include the suffix Main.",
    required: false,
    parse: async (circuitName: string) => {
      if (!isCamelCase(circuitName))
        throw new Error(
          `Circuit name must be camel case. ${circuitName} is not valid.`
        );
      return circuitName;
    },
    multiple: true,
  }),
  linkedCircuitLibraries: Flags.string({
    description:
      "Name of a (parent) directory which contains .circom files. These files can be imported in the circuit which is being compiled.",
    char: "l",
    required: false,
    multiple: true,
  }),
  skipLinkCircomlib: Flags.boolean({
    description: "Omits the linking of the circomlib library.",
    required: false,
    default: false,
  }),
};

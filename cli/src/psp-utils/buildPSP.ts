import { executeAnchor, executeMacroCircom } from "./toolchain";
import { findFile } from "./utils";
import { compileCircuit } from "./buildCircom";
import { Flags } from "@oclif/core";
import { isCamelCase } from "@lightprotocol/zk.js";
import { findAnchorPrograms } from "./addCircuit";

const suffix = "Main.circom";

/**
 * Builds a Private Solana Program (PSP) given a circuit directory.
 * Creates circom files, builds the circom circuit, and compiles the anchor program.
 * @returns {Promise<void>}
 */
export async function buildPSP({
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
  ptau: number;
  programName?: string;
  skipAnchor?: boolean;
  skipCircuit?: boolean;
  skipMacroCircom?: boolean;
  circuitName?: string[];
  linkedCircuitLibraries?: string[];
  skipLinkCircuitlib?: boolean;
  skipLinkCircomlib?: boolean;
}) {
  const compileProgramCircuits = async (
    baseDir: string,
    programName: string,
  ) => {
    const baseDirCircuit = `circuits/${programName}`;
    baseDir = `circuits/`;
    let foundCircuitNames: string[] = [];
    if (!skipCircuit) {
      if (!skipMacroCircom) {
        const circuits = findFile({
          directory: baseDirCircuit,
          extension: ".light",
        });
        for (const { fullPath } of circuits) {
          console.log("📜 Generating circom files");
          console.log("fullPath ", fullPath);
          await executeMacroCircom({
            args: [fullPath, programName],
          });
          console.log("✅ Circom files generated successfully");
        }
      }
      const circuits = findFile({
        directory: baseDirCircuit,
        extension: "Main.circom",
      });
      for (const { filename } of circuits) {
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
        `node_modules/@lightprotocol/circuit-lib.circom/src/light-utils`,
      );
      linkedCircuitLibraries.push(
        `node_modules/@lightprotocol/circuit-lib.circom/src/merkle-tree`,
      );
    }

    if (foundCircuitNames.length > 0) {
      for (const foundCircuitName of foundCircuitNames) {
        // if circuitName is provided skip circuits which have not been provided in the circuitName flag
        if (circuitName && circuitName.indexOf(foundCircuitName) === -1)
          continue;

        console.log("🔑 Compiling circuit ", foundCircuitName);
        await compileCircuit({
          circuitName: foundCircuitName,
          ptau,
          linkedCircuitLibraries,
          programName,
        });
        console.log(`✅ Circuit ${foundCircuitName} generated successfully`);
      }
    } else {
      throw new Error("No circuit found");
    }
  };
  if (programName) {
    await compileProgramCircuits(`./programs/${programName}`, programName);
  } else {
    const { baseDir, programs } = findAnchorPrograms();

    for (const program of programs) {
      const circuitDir = `${baseDir}/circuits/${program}`;
      await compileProgramCircuits(circuitDir, program);
    }
  }

  if (skipAnchor) return;
  console.log("🛠  Building on-chain program");
  await executeAnchor({ args: ["build"] });
  console.log("✅ Build finished successfully");
}

export const buildFlags = {
  name: Flags.string({ description: "Name of the project." }),
  ptau: Flags.integer({ description: "Ptau value.", default: 15 }),
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
          `Circuit name must be camel case. ${circuitName} is not valid.`,
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
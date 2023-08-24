
import { snakeCase } from "snake-case";
import { executeCargoGenerate } from "./toolchain";
import * as path from "path";
import { PSP_DEFAULT_PROGRAM_ID, PSP_TEMPLATE_TAG } from "./constants";
import { camelToScreamingSnake } from "../utils";
import { toCamelCase } from "./index";

export const addCircuit = async ({
    name,
    circom,
  }: {
    name: string;
    circom?: boolean;
  }) => {
    var circomName = snakeCase(name);
    var rustName = snakeCase(name);
    let circuit_template = circom
      ? "psp-template/circuits/circuit_circom"
      : "psp-template/circuits/circuit_psp";
  
    await executeCargoGenerate({
      args: [
        "generate",
        // "--git",
        // "https://github.com/Lightprotocol/psp-template",
        // --tag,
        // PSP_TEMPLATE_TAG,
        "--path",
        "/home/ananas/test_light/psp-template",
        circuit_template,
        "--name",
        name,
        "--define",
        `circom-name=${circomName}`,
        "--define",
        `rust-name=${rustName}`,
        "--define",
        `program-id=${PSP_DEFAULT_PROGRAM_ID}`,
        "--define",
        `VERIFYING_KEY_NAME=${camelToScreamingSnake(circomName)}`,
        "--define",
        `circom-name-camel-case=${toCamelCase(circomName)}`,
        "--vcs",
        "none",
        "--destination",
        `${process.cwd()}/circuits`,
        "--force",
      ],
    });
  };
  
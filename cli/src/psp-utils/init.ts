import { snakeCase } from "snake-case";
import { executeCommandInDir } from "./process";
import { executeCargoGenerate } from "./toolchain";
import * as path from "path";
import { PSP_DEFAULT_PROGRAM_ID, PSP_TEMPLATE_TAG } from "./constants";
import { camelToScreamingSnake } from "../utils";
import { renameFolder } from "./utils";
import { toCamelCase } from "./index";

export enum ProjectType {
    PSP = "psp",
    CIRCOM = "circom",
    PSP_CIRCOM = "psp-circom",
  }

export const initRepo = async (name: string, type: ProjectType) => {
    var circomName = snakeCase(name);
    var rustName = snakeCase(name);
    let programsType = type === ProjectType.PSP_CIRCOM ? ProjectType.PSP : type;
  
    await executeCargoGenerate({
      args: [
        "generate",
        // "--git",
        // "https://github.com/Lightprotocol/psp-template",
        // --tag,
        // PSP_TEMPLATE_TAG,
        "--path",
        "/home/ananas/test_light/psp-template",
        "psp-template",
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
        `type=${type}`,
        "--define",
        `circom-name-camel-case=${toCamelCase(circomName)}`,
        "--define",
        `type-prefix=${programsType}`,
      ],
    });
    type = type === ProjectType.PSP_CIRCOM ? ProjectType.CIRCOM : type;
    await renameFolder(
      `${process.cwd()}/${name}/circuits/circuit_${type}`,
      `${process.cwd()}/${name}/circuits/${name}`
    );
    await renameFolder(
      `${process.cwd()}/${name}/tests_${programsType}`,
      `${process.cwd()}/${name}/tests`
    );
    await renameFolder(
      `${process.cwd()}/${name}/programs_${programsType}`,
      `${process.cwd()}/${name}/programs`
    );
  
    await executeCommandInDir("yarn", ["install"], name);
  };
  
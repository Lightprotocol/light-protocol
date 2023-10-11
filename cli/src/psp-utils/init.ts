import { executeCommandInDir } from "./process";
import { executeCargoGenerate } from "./toolchain";
import { Flags } from '@oclif/core';
import {
  CIRCUIT_LIB_CIRCOM_VERSION,
  LIGHT_MACROS_VERSION,
  LIGHT_SYSTEM_PROGRAM,
  LIGHT_SYSTEM_PROGRAMS_VERSION,
  LIGHT_VERIFIER_SDK_VERSION,
  PROVER_JS_VERSION,
  PSP_DEFAULT_PROGRAM_ID,
  ZK_JS_VERSION,
} from "./constants";
import { renameFolder } from "./utils";
import {
  toSnakeCase,
  toCamelCase,
  camelToScreamingSnake,
} from "@lightprotocol/zk.js";

export enum ProjectType {
  PSP = "psp",
  CIRCOM = "circom",
  PSP_CIRCOM = "psp-circom",
}

export const initRepo = async (name: string, type: ProjectType, flags: any) => {
  const circomName = toSnakeCase(name);
  const rustName = toSnakeCase(name);
  const programsType = type === ProjectType.PSP_CIRCOM ? ProjectType.PSP : type;

  await executeCargoGenerate({
    args: [
      "generate",
      "--git",
      "https://github.com/Lightprotocol/psp-template",
      // TODO(vadorovsky): Switch back to a new release when
      // https://github.com/Lightprotocol/psp-template/pull/12
      // is merged and released.
      // "--tag",
      // PSP_TEMPLATE_TAG,
      "--branch",
      "jorrit/refactor-for-circuit-lib",
      "psp-template",
      "--name",
      name,
      "--define",
      `circom-name=${circomName}`,
      "--define",
      `rust-name=${rustName}`,
      "--define",
      `program-id=${flags.pspDefaultProgramId}`,
      "--define",
      `VERIFYING_KEY_NAME=${camelToScreamingSnake(circomName)}`,
      "--define",
      `type=${type}`,
      "--define",
      `circom-name-camel-case=${toCamelCase(circomName)}`,
      "--define",
      `type-prefix=${programsType}`,
      "--define",
      `zk-js-version=${flags.zkJsVersion}`,
      "--define",
      `prover-js-version=${flags.proverJsVersion}`,
      "--define",
      `circuit-lib-circom-version=${flags.circuitLibCircomVersion}`,
      "--define",
      `light-merkle-tree-program-version=${flags.lightMerkleTreeProgramVersion}`,
      "--define",
      `light-system-program-version=${flags.lightSystemProgramsVersion}`,
      "--define",
      `light-system-program=${flags.lightSystemProgram}`,
      "--define",
      `light-macros-version=${flags.lightMacrosVersion}`,
      "--define",
      `light-verifier-sdk-version=${flags.lightVerifierSdkVersion}`,
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

  await executeCommandInDir("pnpm", ["install"], name);
};


export const cliFlags = {
  zkJsVersion: Flags.string({
    char: 'z',
    description: 'ZK JS version',
    default: ZK_JS_VERSION,
  }),

  proverJsVersion: Flags.string({
    char: 'p',
    description: 'Prover JS version',
    default: PROVER_JS_VERSION,
  }),

  circuitLibCircomVersion: Flags.string({
    char: 'c',
    description: 'Circuit Lib Circom version',
    default: CIRCUIT_LIB_CIRCOM_VERSION,
  }),

  pspDefaultProgramId: Flags.string({
    char: 'i',
    description: 'PSP default program ID',
    default: PSP_DEFAULT_PROGRAM_ID,
  }),

  lightSystemProgramsVersion: Flags.string({
    char: 'l',
    description: 'Light System Programs version',
    default: LIGHT_SYSTEM_PROGRAMS_VERSION,
  }),

  lightSystemProgram: Flags.string({
    char: 's',
    description: 'Light System Program',
    default: LIGHT_SYSTEM_PROGRAM,
  }),

  lightMacrosVersion: Flags.string({
    char: 'm',
    description: 'Light Macros version',
    default: LIGHT_MACROS_VERSION,
  }),

  lightVerifierSdkVersion: Flags.string({
    char: 'v',
    description: 'Light Verifier SDK version',
    default: LIGHT_VERIFIER_SDK_VERSION,
  }),
};

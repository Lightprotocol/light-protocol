import { executeCommandInDir } from "./process";
import { executeCargoGenerate } from "./toolchain";
import { Flags } from "@oclif/core";
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

  // TODO(@ananas-block): switch default to tag once we have a new rust release
  const templateSource = flags.path
    ? ["--path", flags.path]
    : [
        ["--git", flags.git],
        flags.tag
          ? ["--tag", flags.tag]
          : flags.branch
          ? ["--branch", flags.branch]
          : ["--branch", "main"],
      ];
  console.log(templateSource);
  await executeCargoGenerate({
    args: [
      "generate",
      ...templateSource,
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
    aliases: ["zkjs"],
    description: "ZK JS version",
    default: ZK_JS_VERSION,
    required: false,
  }),

  proverJsVersion: Flags.string({
    aliases: ["pjs"],
    description: "Prover JS version",
    default: PROVER_JS_VERSION,
    required: false,
  }),

  circuitLibCircomVersion: Flags.string({
    aliases: ["clib"],
    description: "Circuit Lib Circom version",
    default: CIRCUIT_LIB_CIRCOM_VERSION,
    required: false,
  }),

  lightMerkleTreeProgramVersion: Flags.string({
    aliases: ["lmtv"],
    description: "Light System Programs version",
    default: LIGHT_SYSTEM_PROGRAMS_VERSION,
    required: false,
  }),

  lightSystemProgramsVersion: Flags.string({
    aliases: ["lspv"],
    description: "Light System Programs version",
    default: LIGHT_SYSTEM_PROGRAMS_VERSION,
    required: false,
  }),

  lightSystemProgram: Flags.string({
    aliases: ["lsp"],
    description: "Light System Program",
    default: LIGHT_SYSTEM_PROGRAM,
    required: false,
  }),

  lightMacrosVersion: Flags.string({
    aliases: ["m"],
    description: "Light Macros version",
    default: LIGHT_MACROS_VERSION,
    required: false,
  }),

  lightVerifierSdkVersion: Flags.string({
    aliases: ["vsdk"],
    description: "Light Verifier SDK version",
    default: LIGHT_VERIFIER_SDK_VERSION,
    required: false,
  }),

  path: Flags.string({
    aliases: ["p"],
    description: "Path of the template repo.",
    required: false,
  }),

  git: Flags.string({
    aliases: ["g"],
    description: "Github url of the template repo",
    default: "https://github.com/Lightprotocol/psp-template",
    required: false,
  }),

  tag: Flags.string({
    aliases: ["t"],
    description: "Tag must be used in conjuction with --git",
    required: false,
  }),

  branch: Flags.string({
    aliases: ["b"],
    description: "Branch must be used in conjuction with --git",
    default: "main",
    required: false,
  }),
};

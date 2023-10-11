import { executeCommandInDir } from "./process";
import { executeCargoGenerate } from "./toolchain";
import {
  CIRCUIT_LIB_CIRCOM_VERSION,
  LIGHT_MACROS_VERSION,
  LIGHT_SYSTEM_PROGRAM,
  LIGHT_SYSTEM_PROGRAMS_VERSION,
  LIGHT_VERIFIER_SDK_VERSION,
  PROVER_JS_VERSION,
  PSP_DEFAULT_PROGRAM_ID,
  PSP_TEMPLATE_TAG,
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

export const initRepo = async (name: string, type: ProjectType) => {
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
      "jorrit/adapt-to-psp4in4out-app-storage",
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
      `zk-js-version=${ZK_JS_VERSION}`,
      "--define",
      `prover-js-version=${PROVER_JS_VERSION}`,
      "--define",
      `circuit-lib-circom-version=${CIRCUIT_LIB_CIRCOM_VERSION}`,
      "--define",
      `light-merkle-tree-program-version=${LIGHT_SYSTEM_PROGRAMS_VERSION}`,
      "--define",
      `light-system-program-version=${LIGHT_SYSTEM_PROGRAMS_VERSION}`,
      "--define",
      `light-system-program=${LIGHT_SYSTEM_PROGRAM}`,
      "--define",
      `light-macros-version=${LIGHT_MACROS_VERSION}`,
      "--define",
      `light-verifier-sdk-version=${LIGHT_VERIFIER_SDK_VERSION}`,
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

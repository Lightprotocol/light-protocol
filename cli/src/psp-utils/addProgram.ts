import { executeCargoGenerate } from "./toolchain";
import { PSP_DEFAULT_PROGRAM_ID } from "./constants";
import { toSnakeCase } from "@lightprotocol/zk.js";
import { findDirectoryAnchorBaseDirectory } from "./addCircuit";

export const addProgram = async ({
  name,
  flags,
}: {
  name: string;
  flags: any;
}) => {
  const circomName = toSnakeCase(name);
  const rustName = toSnakeCase(name);
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
  // check that you are in Anchor toml base directory
  const baseDir = findDirectoryAnchorBaseDirectory(process.cwd());

  await executeCargoGenerate({
    args: [
      "generate",
      ...templateSource,
      "psp-template/programs_psp/program_name",
      "--name",
      name,
      "--define",
      `circom-name=${circomName}`,
      "--define",
      `rust-name=${rustName}`,
      "--define",
      `program-id=${PSP_DEFAULT_PROGRAM_ID}`,
      "--define",
      `VERIFYING_KEY_NAME=PLACE_HOLDER`,
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
      "--vcs",
      "none",
      "--destination",
      `${baseDir}/programs/`,
      "--force",
    ],
  });
};

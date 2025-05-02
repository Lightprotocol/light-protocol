import { sleep, UtilsError, UtilsErrorCode } from "@lightprotocol/stateless.js";
import { Args, Command, Flags } from "@oclif/core";
import { executeCommand } from "../../utils/process";
import { downloadBinIfNotExists } from "../../psp-utils";
import {
  PROGRAM_ID,
  ANCHOR_VERSION,
  BORSH_VERSION,
  LIGHT_HASHER_VERSION,
  LIGHT_MACROS_VERSION,
  LIGHT_SDK_VERSION,
  LIGHT_VERIFIER_VERSION,
  SOLANA_SDK_VERSION,
  LIGHT_CLIENT_VERSION,
  LIGHT_TEST_UTILS_VERSION,
  SOLANA_PROGRAM_TEST_VERSION,
  TOKIO_VERSION,
  COMPRESSED_PROGRAM_TEMPLATE_TAG,
  LIGHT_COMPRESSED_ACCOUNT_VERSION,
} from "../../utils/constants";
import {
  CARGO_GENERATE_TAG,
  programFilePath,
  programsDirPath,
} from "../../utils";
import {
  pascalCase as upperCamelCase,
  kebabCase,
  snakeCase,
} from "case-anything";
import { execSync } from "child_process";
export default class InitCommand extends Command {
  static description = "Initialize a compressed account project.";

  static args = {
    name: Args.string({
      name: "NAME",
      description: "The name of the project",
      required: true,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(InitCommand);
    const { name } = args;

    this.log("Initializing project...");
    await initRepo(name, flags);
  }
}

export const initRepo = async (name: string, flags: any) => {
  const localFilePath = programFilePath("cargo-generate");
  const dirPath = programsDirPath();

  await downloadCargoGenerateIfNotExists({
    localFilePath,
    dirPath,
  });

  const kebabCaseName = kebabCase(name);
  const snakeCaseName = snakeCase(name);
  const camelCaseName = upperCamelCase(name);

  const command = localFilePath;
  const env = { ...process.env };

  await executeCommand({
    command,
    args: [
      "generate",
      "--name",
      kebabCaseName,
      "--git",
      "https://github.com/Lightprotocol/compressed-program-template",
      // "--tag",
      // COMPRESSED_PROGRAM_TEMPLATE_TAG,
      "--branch",
      "jorrit/refactor-to-light-sdk-v2",
      "--define",
      `rust-name=${kebabCaseName}`,
      "--define",
      `rust-name-snake-case=${snakeCaseName}`,
      "--define",
      `rust-name-camel-case=${camelCaseName}`,
      "--define",
      `program-id=${PROGRAM_ID}`,
      "--define",
      `anchor-version=${ANCHOR_VERSION}`,
      "--define",
      `borsh-version=${BORSH_VERSION}`,
      "--define",
      `light-hasher-version=${LIGHT_HASHER_VERSION}`,
      "--define",
      `light-macros-version=${LIGHT_MACROS_VERSION}`,
      "--define",
      `light-sdk-version=${LIGHT_SDK_VERSION}`,
      "--define",
      `light-compressed-account-version=${LIGHT_COMPRESSED_ACCOUNT_VERSION}`,
      "--define",
      `light-verifier-version=${LIGHT_VERIFIER_VERSION}`, // TODO: remove
      "--define",
      `solana-sdk-version=${SOLANA_SDK_VERSION}`,
      "--define",
      `light-client-version=${LIGHT_CLIENT_VERSION}`,
      "--define",
      `light-test-utils-version=${LIGHT_TEST_UTILS_VERSION}`,
      "--define",
      `light-program-test-version=${SOLANA_PROGRAM_TEST_VERSION}`,
      "--define",
      `solana-program-test-version=${SOLANA_PROGRAM_TEST_VERSION}`, // TODO: remove
      "--define",
      `tokio-version=${TOKIO_VERSION}`,
    ],
    logFile: true,
    env: env,
  });

  console.log("✅ Project initialized successfully");

  if (os.platform() === "darwin") {
    console.log(`
🧢 Important for macOS users 🧢
===============================

Run this command in your terminal before building your project:

----------------------------------------------------------------------------------------------------
echo 'export CPATH="/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include:$CPATH"' >> ~/.zshrc && source ~/.zshrc
----------------------------------------------------------------------------------------------------
`);
  }

  await sleep(1000);
};

export const initFlags = {
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

export async function downloadCargoGenerateIfNotExists({
  localFilePath,
  dirPath,
}: {
  localFilePath: string;
  dirPath: string;
}) {
  let remoteFileName: string;
  const tag = CARGO_GENERATE_TAG;
  switch (getSystem()) {
    case System.LinuxAmd64:
      remoteFileName = `cargo-generate-${tag}-x86_64-unknown-linux-musl.tar.gz`;
      break;
    case System.LinuxArm64:
      remoteFileName = `cargo-generate-${tag}-aarch64-unknown-linux-musl.tar.gz`;
      break;
    case System.MacOsAmd64:
      remoteFileName = `cargo-generate-${tag}-x86_64-apple-darwin.tar.gz`;
      break;
    case System.MacOsArm64:
      remoteFileName = `cargo-generate-${tag}-aarch64-apple-darwin.tar.gz`;
      break;
    default:
      throw new Error(`Unsupported system: ${getSystem()}`);
  }

  await downloadBinIfNotExists({
    localFilePath,
    dirPath,
    owner: "cargo-generate",
    repoName: "cargo-generate",
    remoteFileName,
    tag,
  });
}
export enum System {
  MacOsAmd64,
  MacOsArm64,
  LinuxAmd64,
  LinuxArm64,
}
import * as os from "os";
export function getSystem(): System {
  const arch = os.arch();
  const platform = os.platform();

  switch (platform) {
    case "darwin":
      switch (arch) {
        case "x64":
          return System.MacOsAmd64;
        case "arm":
        // fallthrough
        case "arm64":
          return System.MacOsArm64;
        default:
          throw new UtilsError(
            UtilsErrorCode.UNSUPPORTED_ARCHITECTURE,
            "getSystem",
            `Architecture ${arch} is not supported.`,
          );
      }
    case "linux":
      switch (arch) {
        case "x64":
          return System.LinuxAmd64;
        case "arm":
        // fallthrough
        case "arm64":
          return System.LinuxArm64;
        default:
          throw new UtilsError(
            UtilsErrorCode.UNSUPPORTED_ARCHITECTURE,
            "getSystem",
            `Architecture ${arch} is not supported.`,
          );
      }
  }

  throw new UtilsError(
    UtilsErrorCode.UNSUPPORTED_PLATFORM,
    "getSystem",
    `Platform ${platform} is not supported.`,
  );
}

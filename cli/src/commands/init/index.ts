import {
  sleep,
  STATE_MERKLE_TREE_NETWORK_FEE,
  UtilsError,
  UtilsErrorCode,
} from "@lightprotocol/stateless.js";
import { Args, Command, Flags } from "@oclif/core";
import { executeCommand } from "../../utils/process";
import { downloadBinIfNotExists } from "../../psp-utils";
import {
  PROGRAM_ID,
  ANCHOR_VERSION,
  LIGHT_HASHER_VERSION,
  LIGHT_MACROS_VERSION,
  LIGHT_SDK_VERSION,
  LIGHT_SDK_MACROS_VERSION,
  SOLANA_SDK_VERSION,
  LIGHT_CLIENT_VERSION,
  TOKIO_VERSION,
  COMPRESSED_PROGRAM_TEMPLATE_TAG,
  LIGHT_COMPRESSED_ACCOUNT_VERSION,
  STATELESS_JS_VERSION,
  LIGHT_CLI_VERSION,
  SOLANA_CLI_VERSION,
  LIGHT_SDK_TYPES_VERSION,
  LIGHT_PROGRAM_TEST_VERSION,
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
      "--tag",
      COMPRESSED_PROGRAM_TEMPLATE_TAG,
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
      `light-hasher-version=${LIGHT_HASHER_VERSION}`,
      "--define",
      `light-macros-version=${LIGHT_MACROS_VERSION}`,
      "--define",
      `light-sdk-types-version=${LIGHT_SDK_TYPES_VERSION}`,
      "--define",
      `light-sdk-version=${LIGHT_SDK_VERSION}`,
      "--define",
      `light-sdk-macros-version=${LIGHT_SDK_MACROS_VERSION}`,
      "--define",
      `solana-sdk-version=${SOLANA_SDK_VERSION}`,
      "--define",
      `light-client-version=${LIGHT_CLIENT_VERSION}`,
      "--define",
      `light-program-test-version=${LIGHT_PROGRAM_TEST_VERSION}`, // TODO: remove
      "--define",
      `tokio-version=${TOKIO_VERSION}`,
      "--define",
      `stateless-js-version=${STATELESS_JS_VERSION}`,
      "--define",
      `anchor-js-version=${ANCHOR_VERSION}`,
      "--define",
      `light-cli-version=${LIGHT_CLI_VERSION}`,
      "--define",
      `solana-cli-version=${SOLANA_CLI_VERSION}`,
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
    description: "Tag must be used in conjunction with --git",
    required: false,
  }),

  branch: Flags.string({
    aliases: ["b"],
    description: "Branch must be used in conjunction with --git",
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

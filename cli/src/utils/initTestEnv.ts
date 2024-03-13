import {
  ADMIN_AUTH_KEY,
  ADMIN_AUTH_KEYPAIR,
  airdropSol,
  sleep,
} from "@lightprotocol/zk.js";
import { setAnchorProvider, setRpcRecipient } from "./utils";
import { Keypair } from "@solana/web3.js";
import {
  LIGHT_MERKLE_TREE_PROGRAM_TAG,
  SPL_NOOP_PROGRAM_TAG,
  downloadBinIfNotExists,
  executeCommand,
} from "../psp-utils";
import path from "path";
const find = require("find-process");

const LIGHT_PROTOCOL_PROGRAMS_DIR_ENV = "LIGHT_PROTOCOL_PROGRAMS_DIR";
const BASE_PATH = "../../bin/";

export async function initTestEnv({
  additionalPrograms,
  skipSystemAccounts,
  background = true,
}: {
  additionalPrograms?: { address: string; path: string }[];
  skipSystemAccounts?: boolean;
  background?: boolean;
}) {
  console.log("Performing setup tasks...\n");

  const initAccounts = async () => {
    await sleep(10000);
    const anchorProvider = await setAnchorProvider();
    await airdropSol({
      connection: anchorProvider.connection,
      lamports: 100e9,
      recipientPublicKey: ADMIN_AUTH_KEYPAIR.publicKey,
    });

    // leaving this here for now maybe we can use it for
    // await createTestAccounts(anchorProvider.connection);

    // const lookupTable = await initLookUpTableFromFile(anchorProvider);

    // setLookUpTable(lookupTable.toString());

    const rpcRecipientSol = Keypair.generate().publicKey;

    setRpcRecipient(rpcRecipientSol.toString());

    await anchorProvider.connection.requestAirdrop(
      rpcRecipientSol,
      2_000_000_000,
    );
  };
  initAccounts();
  if (!background) {
    await startTestValidator({ additionalPrograms, skipSystemAccounts });
  } else {
    startTestValidator({ additionalPrograms, skipSystemAccounts });
    await sleep(15000);
  }
}

export async function initTestEnvIfNeeded({
  additionalPrograms,
  skipSystemAccounts,
}: {
  additionalPrograms?: { address: string; path: string }[];
  skipSystemAccounts?: boolean;
} = {}) {
  try {
    const anchorProvider = await setAnchorProvider();
    // this request will fail if there is no local test validator running
    await anchorProvider.connection.getBalance(ADMIN_AUTH_KEY);
  } catch (error) {
    // launch local test validator and initialize test environment
    await initTestEnv({
      additionalPrograms,
      skipSystemAccounts,
      background: true,
    });
  }
}

/*
 * Determines a path to which Light Protocol programs should be downloaded.
 *
 * If the `LIGHT_PROTOCOL_PROGRAMS_DIR` environment variable is set, the path
 * provided in it is used.
 *
 * Otherwise, the `bin` directory in the CLI internals is used.
 *
 * @returns {string} Directory path for Light Protocol programs.
 */
function programsDirPath(): string {
  return (
    process.env[LIGHT_PROTOCOL_PROGRAMS_DIR_ENV] ||
    path.resolve(__dirname, BASE_PATH)
  );
}

/*
 * Determines a patch to which the given program should be downloaded.
 *
 * If the `LIGHT_PROTOCOL_PROGRAMS_DIR` environment variable is set, the path
 * provided in it is used as a parent
 *
 * Otherwise, the `bin` directory in the CLI internals is used.
 *
 * @returns {string} Path for the given program.
 */
function programFilePath(programName: string): string {
  const programsDir = process.env[LIGHT_PROTOCOL_PROGRAMS_DIR_ENV];
  if (programsDir) {
    return path.join(programsDir, programName);
  }

  return path.resolve(__dirname, path.join(BASE_PATH, programName));
}

export async function getSolanaArgs({
  additionalPrograms,
  skipSystemAccounts,
}: {
  additionalPrograms?: { address: string; path: string }[];
  skipSystemAccounts?: boolean;
}): Promise<Array<string>> {
  const LIMIT_LEDGER_SIZE = "500000000";

  type Program = { id: string; name?: string; tag?: string; path?: string };
  // TODO: adjust program tags
  const programs: Program[] = [
    {
      id: "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV",
      name: "spl_noop.so",
      tag: SPL_NOOP_PROGRAM_TAG,
    },
  ];
  if (additionalPrograms)
    additionalPrograms.forEach((program) => {
      programs.push({ id: program.address, path: program.path });
    });

  const dirPath = programsDirPath();

  const solanaArgs = [
    "--reset",
    `--limit-ledger-size=${LIMIT_LEDGER_SIZE}`,
    "--quiet",
  ];

  for (const program of programs) {
    if (program.path) {
      solanaArgs.push("--bpf-program", program.id, program.path);
    } else {
      const localFilePath = programFilePath(program.name!);
      await downloadBinIfNotExists({
        localFilePath,
        dirPath,
        owner: "Lightprotocol",
        repoName: "light-protocol",
        remoteFileName: program.name!,
        tag: program.tag,
      });
      solanaArgs.push("--bpf-program", program.id, localFilePath);
    }
  }
  if (!skipSystemAccounts) {
    const accountsRelPath = "../../accounts";
    const accountsPath = path.resolve(__dirname, accountsRelPath);
    solanaArgs.push("--account-dir", accountsPath);
  }

  return solanaArgs;
}

export async function startTestValidator({
  additionalPrograms,
  skipSystemAccounts,
}: {
  additionalPrograms?: { address: string; path: string }[];
  skipSystemAccounts?: boolean;
}) {
  const command = "solana-test-validator";
  const solanaArgs = await getSolanaArgs({
    additionalPrograms,
    skipSystemAccounts,
  });

  await killTestValidator();

  await new Promise((r) => setTimeout(r, 1000));

  await executeCommand({
    command,
    args: [...solanaArgs],
  });
}

export async function killTestValidator() {
  const processList = await find("name", "solana-test-validator");

  for (const proc of processList) {
    process.kill(proc.pid);
  }
}

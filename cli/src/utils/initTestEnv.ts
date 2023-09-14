import {
  ADMIN_AUTH_KEY,
  ADMIN_AUTH_KEYPAIR,
  airdropSol,
  createTestAccounts,
  initLookUpTableFromFile,
  sleep,
} from "@lightprotocol/zk.js";
import {
  setAnchorProvider,
  setLookUpTable,
  setRelayerRecipient,
} from "./utils";
import { Keypair } from "@solana/web3.js";
import { downloadBinIfNotExists, executeCommand } from "../psp-utils";
import path from "path";
import { PROGRAM_TAG } from "../psp-utils";
const find = require("find-process");

export async function initTestEnv({
  additonalPrograms,
  skip_system_accounts,
  background,
}: {
  additonalPrograms?: { address: string; path: string }[];
  skip_system_accounts?: boolean;
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

    await createTestAccounts(anchorProvider.connection);

    const lookupTable = await initLookUpTableFromFile(anchorProvider);

    setLookUpTable(lookupTable.toString());

    const relayerRecipientSol = Keypair.generate().publicKey;

    setRelayerRecipient(relayerRecipientSol.toString());

    await anchorProvider.connection.requestAirdrop(
      relayerRecipientSol,
      2_000_000_000
    );
  };
  initAccounts();
  if (!background) {
    await start_test_validator({ additonalPrograms, skip_system_accounts });
  } else {
    start_test_validator({ additonalPrograms, skip_system_accounts });
    await sleep(15000);
  }
}

export async function initTestEnvIfNeeded({
  additonalPrograms,
  skip_system_accounts,
}: {
  additonalPrograms?: { address: string; path: string }[];
  skip_system_accounts?: boolean;
} = {}) {
  try {
    const anchorProvider = await setAnchorProvider();
    // this request will fail if there is no local test validator running
    await anchorProvider.connection.getBalance(ADMIN_AUTH_KEY);
  } catch (error) {
    // launch local test validator and initialize test environment
    await initTestEnv({
      additonalPrograms,
      skip_system_accounts,
      background: true,
    });
  }
}

export async function start_test_validator({
  additonalPrograms,
  skip_system_accounts,
}: {
  additonalPrograms?: { address: string; path: string }[];
  skip_system_accounts?: boolean;
}) {
  const command = "solana-test-validator";
  const LIMIT_LEDGER_SIZE = "500000000";
  const BASE_PATH = "../../bin/";
  type Program = { id: string; name?: string; path?: string };
  const programs: Program[] = [
    { id: "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV", name: "spl_noop.so" },
    {
      id: "JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6",
      name: "merkle_tree_program.so",
    },
    {
      id: "J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i",
      name: "verifier_program_zero.so",
    },
    {
      id: "DJpbogMSrK94E1zvvJydtkqoE4sknuzmMRoutd6B7TKj",
      name: "verifier_program_storage.so",
    },
    {
      id: "J85SuNBBsba7FQS66BiBCQjiQrQTif7v249zL2ffmRZc",
      name: "verifier_program_one.so",
    },
    {
      id: "2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86",
      name: "verifier_program_two.so",
    },
  ];
  if (additonalPrograms)
    additonalPrograms.forEach((program) => {
      programs.push({ id: program.address, path: program.path });
    });

  const dirPath = path.resolve(__dirname, BASE_PATH);

  let solanaArgs = [
    "--reset",
    `--limit-ledger-size=${LIMIT_LEDGER_SIZE}`,
    "--quiet",
  ];

  for (let program of programs) {
    let filePathString = BASE_PATH + program.name;
    const localFilePath = path.resolve(__dirname, filePathString);
    if (!program.path) {
      // TODO: add tag
      await downloadBinIfNotExists({
        localFilePath,
        dirPath,
        owner: "Lightprotocol",
        repoName: "light-protocol",
        remoteFileName: program.name!,
        tag: PROGRAM_TAG,
      });
    }

    let path1 = program.path ? program.path : `${localFilePath}`;
    solanaArgs.push("--bpf-program", program.id, path1);
  }
  let dirPathString = "../../accounts/";
  const localFilePath = path.resolve(__dirname, dirPathString);
  if (!skip_system_accounts) {
    solanaArgs.push("--account-dir", localFilePath);
  }

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

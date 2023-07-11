import { Command, Flags } from "@oclif/core";
import { Keypair as SolanaKeypair } from "@solana/web3.js";
import { exec } from "child_process";
import {
  createTestAccounts,
  initLookUpTableFromFile,
  sleep,
} from "@lightprotocol/zk.js";
import {
  setRelayerRecipient,
  setAnchorProvider,
  setLookUpTable,
  CustomLoader,
} from "../../utils/utils";
import { initTestEnv } from "../../utils/initTestEnv";
import { executeCommand } from "../../utils/process";

class SetupCommand extends Command {
  static description = "Perform setup tasks";

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }
  // {bpf_program_id: string, path: string}
  static flags = {
    bpf_program: Flags.string({
      aliases: ["bp"],
      description:
        "Solana bpf program whill be deployed on local test validator <ADDRESS_OR_KEYPAIR> <SBF_PROGRAM.SO>",
    }),
    kill: Flags.boolean({
      aliases: ["k"],
      description: "Kills a running test validator",
      hidden: true,
      default: true,
    }),
    // kill flag kills a running validator and doesn't start a new one
    // dev starts with programs fetched without light protocol repo
  };

  async run() {
    const { flags } = await this.parse(SetupCommand);
    const { bpfProgram = [], kill } = flags;
    const limitLedgerSize = 500000000,
      faucetPort = 9002,
      rpcPort = 8899,
      accountDir = "../test-env/accounts";
    const loader = new CustomLoader("Performing setup tasks...\n");
    loader.start();
    this.log("flags.kill is: ", kill);
    if (flags.kill) {
      try {
        await executeCommand({
          command: "docker",
          args: ["rm", "-f", "solana-validator"],
        });
        this.log("Killed test validator");
      } catch (error) {
        this.log("No test validator running");
      }
    }
    // const standardPrograms: string[] = [
    //   "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV:../test-env/programs/spl_noop.so",
    //   "JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6:../light-system-programs/target/deploy/merkle_tree_program.so",
    //   "J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i:../light-system-programs/target/deploy/verifier_program_zero.so",
    //   "DJpbogMSrK94E1zvvJydtkqoE4sknuzmMRoutd6B7TKj:../light-system-programs/target/deploy/verifier_program_storage.so",
    //   "J85SuNBBsba7FQS66BiBCQjiQrQTif7v249zL2ffmRZc:../light-system-programs/target/deploy/verifier_program_one.so"
    // ];

    // const allPrograms = [...standardPrograms, ...bpfProgram];

    // const programIdSoFilePathPairs = allPrograms.map((program: string) => {
    //   const [programId, soFilePath] = program.split(':');
    //   return ["--bpf-program", programId, soFilePath];
    // });

    // const args = [
    //   "--reset",
    //   "--limit-ledger-size", limitLedgerSize.toString(),
    //   "--faucet-port", faucetPort.toString(),
    //   "--rpc-port", rpcPort.toString(),
    //   "--quiet",
    //   "--account-dir", accountDir,
    //   ...programIdSoFilePathPairs.flat()
    // ];

    // try {
    //   executeCommand({
    //     command: "solana-test-validator",
    //     args,
    //   });
    //   await sleep(9000);
    //   this.log("\nSetup script executed successfully solana test validator with light programs is running in the background");
    // } catch (error) {

    try {
      await initTestEnv();
      this.log("\nSetup tasks completed successfully \x1b[32m✔\x1b[0m");
      loader.stop(false);
    } catch (error) {
      this.error(`\nSetup tasks failed: ${error}`);
    }
    loader.stop(false);
  }
}

export default SetupCommand;

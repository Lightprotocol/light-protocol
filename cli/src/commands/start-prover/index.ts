import { Command, Flags } from "@oclif/core";
import { CustomLoader } from "../../utils/index";
import { healthCheck, startProver } from "../../utils/processProverServer";

class StartProver extends Command {
  static description = "Start gnark prover";

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static flags = {
    "prover-port": Flags.integer({
      description: "Enable Light Prover server on this port.",
      required: false,
      default: 3001,
    }),
    "run-mode": Flags.string({
      description:
        "Specify the running mode (local-rpc, forester, forester-test, rpc, or full). Default: local-rpc",
      options: [
        "local-rpc",
        "rpc",
        "forester",
        "forester-test",
        "full",
        "full-test",
      ],
      required: false,
    }),
    circuit: Flags.string({
      description: "Specify individual circuits to enable.",
      options: [
        "inclusion",
        "non-inclusion",
        "combined",
        "append",
        "update",
        "address-append",
        "append-test",
        "update-test",
        "address-append-test",
      ],
      multiple: true,
      required: false,
    }),
    force: Flags.boolean({
      description:
        "Force restart the prover even if one is already running with the same flags.",
      required: false,
      default: false,
    }),
    redisUrl: Flags.string({
      description:
        "Redis URL to use for the prover (e.g. redis://localhost:6379)",
      required: false,
    }),
  };

  async run() {
    const { flags } = await this.parse(StartProver);
    const loader = new CustomLoader("Performing setup tasks...\n");
    loader.start();

    if (!flags["run-mode"] && !flags["circuit"]) {
      this.log("Please specify --run-mode or --circuit.");
      return;
    }

    const proverPort = flags["prover-port"] || 3001;
    const force = flags["force"] || false;
    const redisUrl = flags["redisUrl"] || process.env.REDIS_URL || undefined;

    // TODO: remove this workaround.
    // Force local-rpc mode when rpc is specified
    let runMode = flags["run-mode"];
    if (runMode === "rpc") {
      runMode = "local-rpc";
      this.log("Note: Running in local-rpc mode instead of rpc mode");
    }

    await startProver(proverPort, runMode, flags["circuit"], force, redisUrl);

    const healthy = await healthCheck(proverPort, 10, 1000);
    loader.stop();
    if (healthy) {
      this.log("\nProver started and passed health check \x1b[32m✔\x1b[0m");
    } else {
      this.log("\nProver started but health check failed");
    }
  }
}

export default StartProver;

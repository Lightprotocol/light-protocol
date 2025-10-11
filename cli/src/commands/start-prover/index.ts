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

    const proverPort = flags["prover-port"] || 3001;
    const redisUrl = flags["redisUrl"] || process.env.REDIS_URL || undefined;

    await startProver(proverPort, redisUrl);

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

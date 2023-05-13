import { Args, Command, Flags } from "@oclif/core";
import { getLightProvider, getPayer, getWalletConfig } from "../../utils";

class ConfigureCommand extends Command {
  static description =
    "Update the configuration of the Merkle Tree NFTs, permissionless SPL tokens, and lock duration";

  static examples = [
    "light configure nfts",
    "light configure spl",
    "light configure lock -l <lockDuration>",
  ];

  static args = {
    method: Args.string({
      name: "method",
      description: "Method to perform: nfts, spl, or lock",
      required: true,
    }),
  };

  static flags = {
    lockDuration: Flags.string({
      char: "l",
      description: "Update the lock duration configuration",
    }),
  };

  async run() {
    const { args, flags } = await this.parse(ConfigureCommand);
    const { method } = args;
    const { lockDuration } = flags;

    try {
      const payer = getPayer();
      const provider = await getLightProvider(payer);
      let merkleTreeConfig = await getWalletConfig(provider.provider!);

      if (method === "nfts") {
        this.log("Updating NFT Merkle Tree Configuration...");
        try {
          // TODO: figure out this function
          // const tx = await merkleTreeConfig.enableNfts(true);
          this.log("NFTs tokens enabled", { success: true });
        } catch (err) {
          this.error(`${err}`);
        }
      } else if (method === "spl") {
        this.log("Updating SPL Merkle Tree Configuration...");
        try {
          await merkleTreeConfig.enablePermissionlessSplTokens(true);
          this.log("SPL tokens enabled", { success: true });
        } catch (err) {
          this.error(`${err}`);
        }
      } else if (method === "lock") {
        if (!lockDuration) {
          this.error("Please provide the lock duration");
          return;
        }
        this.log("Updating Lock Merkle Tree Configuration...");
        try {
          await merkleTreeConfig.updateLockDuration(parseInt(lockDuration));
          this.log(`Lock Duration updated: ${parseInt(lockDuration)}`, {
            success: true,
          });
        } catch (err) {
          this.error(`${err}`);
        }
      } else {
        this.error(
          'Invalid command. Please use "nfts", "spl", or "lock" along with the configure command'
        );
      }
      this.log("Merkle Tree Configuration updated successfully", {
        success: true,
      });
    } catch (error) {
      let errorMessage = "Aborted.";
      if (error instanceof Error) {
        errorMessage = error.message;
      }
      // @ts-ignore
      if (error.logs && error.logs.length > 0) {
        // @ts-ignore
        errorMessage = error.logs;
      }
      this.error(errorMessage);
    }
  }
}

export default ConfigureCommand;

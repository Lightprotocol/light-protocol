import { Args, Command, Flags } from "@oclif/core";
import * as anchor from "@coral-xyz/anchor";
import {
  getLocalProvider,
  getWalletConfig,
  readPayerFromIdJson,
} from "../../utils";

class ConfigureCommand extends Command {
  static description =
    "Update the configuration of the Merkle Tree NFTs, permissionless SPL tokens, and lock duration";

  static examples = [
    "light-cli configure nfts",
    "light-cli configure spl",
    "light-cli configure lock -l <lockDuration>",
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
      const payer = new anchor.Wallet(readPayerFromIdJson());
      const provider = await getLocalProvider(payer);
      let merkleTreeConfig = await getWalletConfig(provider);

      if (method === "nfts") {
        this.log("Updating NFT Merkle Tree Configuration...");
        try {
          const tx = await merkleTreeConfig.enableNfts(true);
          this.log("NFTs tokens enabled", { success: true });
        } catch (err) {
          this.error(err.message);
        }
      } else if (method === "spl") {
        this.log("Updating SPL Merkle Tree Configuration...");
        try {
          await merkleTreeConfig.enablePermissionlessSplTokens(true);
          this.log("SPL tokens enabled", { success: true });
        } catch (err) {
          this.error(err.message);
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
          this.error(err.message);
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
      if (error.logs && error.logs.length > 0) {
        errorMessage = error.logs;
      }
      this.error(errorMessage);
    }
  }
}


export default ConfigureCommand;

import { Args, Command, Flags } from "@oclif/core";
import { getLoader, getWalletConfig, setAnchorProvider } from "../../utils";

class ConfigureCommand extends Command {
  static description =
    "Update the configuration of the Merkle Tree NFTs, permissionless SPL tokens, and lock duration";

  static examples = [
    "light configure spl",
    "light configure lock -l <lockDuration>",
  ];

  static args = {
    method: Args.string({
      name: "method",
      description: "Method to perform: spl or lock",
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

    const { loader, end } = getLoader("Updating Merkle Tree configuration...");

    try {
      const { connection } = await setAnchorProvider();

      let merkleTreeConfig = await getWalletConfig(connection);

      if (method === "spl") {
        try {
          let merkleTreeAuthority =
            await merkleTreeConfig.merkleTreeProgram.account.merkleTreeAuthority.fetch(
              merkleTreeConfig.merkleTreeAuthorityPda!
            );

          const enablePermissionlessSplTokens =
            !merkleTreeAuthority.enablePermissionlessSplTokens;

          await merkleTreeConfig.enablePermissionlessSplTokens(
            enablePermissionlessSplTokens
          );
          this.log(
            `Permissionless SPL tokens ${
              enablePermissionlessSplTokens ? "enabled" : "disabled"
            }`
          );
        } catch (err) {
          this.error(`Failed to update SPL token configuration: ${err}`);
        }
      } else if (method === "lock") {
        if (!lockDuration) {
          this.error("Please provide the lock duration");
        }
        try {
          await merkleTreeConfig.updateLockDuration(parseInt(lockDuration));
          this.log(`Lock duration updated: ${parseInt(lockDuration)} seconds`);
        } catch (err) {
          this.error(`Failed to update lock duration configuration: ${err}`);
        }
      } else {
        this.error('Invalid command. Please use "spl" or "lock"');
      }
      end(loader);
    } catch (error) {
      end(loader);
      this.error(`Failed to update Merkle Tree configuration: ${error}`);
    }
  }
}

export default ConfigureCommand;

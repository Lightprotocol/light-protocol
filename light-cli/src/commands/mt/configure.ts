import { Args, Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  getWalletConfig,
  setAnchorProvider,
} from "../../utils/utils";

class ConfigureCommand extends Command {
  static description =
    "Update the configuration of the Merkle Tree NFTs, permissionless SPL tokens, and lock duration";

  static examples = [
    "light configure spl",
    "light configure lock -l <lockDuration>",
    "light configure show",
  ];

  static args = {
    method: Args.string({
      name: "method",
      description: "Method to perform: spl or lock",
      required: true,
    }),
  };

  static flags = {
    duration: Flags.string({
      char: "d",
      description: "Update the lock duration configuration",
    }),
  };

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  async run() {
    const { args, flags } = await this.parse(ConfigureCommand);
    const { method } = args;
    const { duration } = flags;

    const loader = new CustomLoader("Updating Merkle Tree configuration...");

    loader.start();

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
            `\nPermissionless SPL tokens ${
              enablePermissionlessSplTokens ? "enabled" : "disabled"
            }`
          );
        } catch (err) {
          this.error(`\nFailed to update SPL token configuration: ${err}`);
        }
      } else if (method === "lock") {
        if (!duration) {
          this.error("\nPlease provide the lock duration");
        }
        try {
          await merkleTreeConfig.updateLockDuration(parseInt(duration));
          this.log(`\nLock duration updated: ${parseInt(duration)} seconds`);
        } catch (err) {
          this.error(`\nFailed to update lock duration configuration: ${err}`);
        }
      } else if (method === "show") {
        try {
          let merkleTreeAuthority =
            await merkleTreeConfig.merkleTreeProgram.account.merkleTreeAuthority.fetch(
              merkleTreeConfig.merkleTreeAuthorityPda!
            );

          this.log(
            `\nPermissionless SPL: ${
              merkleTreeAuthority.enablePermissionlessSplTokens
                ? "enabled"
                : "disabled"
            }`
          );

          this.log(
            `Lock Duration: ${merkleTreeAuthority.lockDuration.toString()}`
          );
        } catch (err) {}
      } else {
        this.error('\nInvalid command. Please use "show" , "spl" or "lock"');
      }
      loader.stop();
    } catch (error) {
      loader.stop();

      this.error(`\nFailed to update Merkle Tree configuration: ${error}`);
    }
  }
}

export default ConfigureCommand;

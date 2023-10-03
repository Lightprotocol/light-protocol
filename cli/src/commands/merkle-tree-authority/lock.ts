import { Args, Command } from "@oclif/core";
import {
  CustomLoader,
  getWalletConfig,
  setAnchorProvider,
} from "../../utils/utils";

class LockCommand extends Command {
  static description = "Update the lock duration.";

  static examples = ["light merkle-tree-authority:lock 100"];

  static args = {
    duration: Args.integer({
      name: "duration",
      description: "Duration to lock the Transaction Merkle Trees for.",
      required: true,
    }),
  };

  async run() {
    const { args } = await this.parse(LockCommand);
    const { duration } = args;

    const loader = new CustomLoader(
      "Updating lock duration of Transaction Merkle Trees"
    );
    loader.start();

    const anchorProvider = await setAnchorProvider();
    let merkleTreeConfig = await getWalletConfig(anchorProvider);

    await merkleTreeConfig.updateLockDuration(duration);

    loader.stop(false);
    this.log("Lock updated successfully \x1b[32m✔\x1b[0m");
  }
}

export default LockCommand;

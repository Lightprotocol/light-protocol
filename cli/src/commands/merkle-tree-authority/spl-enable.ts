import { Command } from "@oclif/core";
import { CustomLoader, getWalletConfig, setAnchorProvider } from "../../utils";

class SplEnableCommand extends Command {
  static description = "Enable permissionless SPL tokens.";

  static examples = ["light merkle-tree-authority:spl"];

  async run() {
    const loader = new CustomLoader("Enabling permissionless SPL tokens");
    loader.start();

    const anchorProvider = await setAnchorProvider();
    const merkleTreeConfig = await getWalletConfig(anchorProvider);

    await merkleTreeConfig.enablePermissionlessSplTokens(true);

    loader.stop(false);
    this.log(
      "Permissionless SPL tokens enabled successfully \x1b[32m✔\x1b[0m",
    );
  }
}

export default SplEnableCommand;

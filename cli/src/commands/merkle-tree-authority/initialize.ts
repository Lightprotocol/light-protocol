import { Command } from "@oclif/core";
import {
  CustomLoader,
  getWalletConfig,
  setAnchorProvider,
} from "../../utils/utils";

class InitializeCommand extends Command {
  static description = "Initialize the Merkle Tree Authority.";

  static examples = ["light merkle-tree-authority:initialize"];

  async run() {
    const loader = new CustomLoader("Initializing Merkle Tree Authority");
    loader.start();

    const { connection } = await setAnchorProvider();
    let merkleTreeConfig = await getWalletConfig(connection);

    const accountInfo = await connection.getAccountInfo(
      merkleTreeConfig.getMerkleTreeAuthorityPda()
    );
    if (accountInfo && accountInfo.data.length > 0) {
      this.log("Merkle Tree Authority already initialized");
    } else {
      await merkleTreeConfig.initMerkleTreeAuthority();
      this.log(
        "Merkle Tree Authority initialized successfully \x1b[32m✔\x1b[0m"
      );
    }
    loader.stop(false);
  }
}

export default InitializeCommand;

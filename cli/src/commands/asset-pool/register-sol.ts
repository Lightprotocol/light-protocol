import { Command } from "@oclif/core";
import { CustomLoader, getWalletConfig, setAnchorProvider } from "../../utils";
import { POOL_TYPE } from "@lightprotocol/zk.js";

class RegisterSolCommand extends Command {
  static description = "Register SOL pool.";

  static examples = ["light asset-pool:register-sol"];

  async run() {
    const loader = new CustomLoader("Registering SOL pool");
    loader.start();

    const { connection } = await setAnchorProvider();
    let merkleTreeConfig = await getWalletConfig(connection);

    await merkleTreeConfig.registerSolPool(POOL_TYPE);

    loader.stop(false);
    this.log("SOL pool registered successfully \x1b[32m✔\x1b[0m");
  }
}

export default RegisterSolCommand;

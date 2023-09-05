import { Args, Command } from "@oclif/core";
import { CustomLoader, getWalletConfig, setAnchorProvider } from "../../utils";
import { POOL_TYPE } from "@lightprotocol/zk.js";
import { PublicKey } from "@solana/web3.js";

class RegisterSplCommand extends Command {
  static description = "Register SPL pool.";

  static examples = ["light asset-pool:register-spl"];

  static args = {
    mint: Args.string({
      name: "mint",
      description: "Solana public key for the mint.",
      required: true,
    }),
  };

  async run() {
    const loader = new CustomLoader("Registering SPL pool");
    loader.start();

    const { args } = await this.parse(RegisterSplCommand);
    const { mint } = args;

    const { connection } = await setAnchorProvider();
    let merkleTreeConfig = await getWalletConfig(connection);

    const mintKey = new PublicKey(mint);

    await merkleTreeConfig.registerSplPool(POOL_TYPE, mintKey);

    loader.stop(false);
    this.log("SPL pool registered successfully \x1b[32m✔\x1b[0m");
  }
}

export default RegisterSplCommand;

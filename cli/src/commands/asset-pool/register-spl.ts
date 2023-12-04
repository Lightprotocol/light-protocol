import { BN } from "@coral-xyz/anchor";
import { Args, Command } from "@oclif/core";
import { CustomLoader, getWalletConfig, setAnchorProvider } from "../../utils";
import { PublicKey } from "@solana/web3.js";

class RegisterSplCommand extends Command {
  static description = "Register SPL pool.";

  static examples = ["light asset-pool:register-spl"];

  static args = {
    poolType: Args.string({
      description: "Pool type to register the SPL pool in.",
      required: true,
    }),
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
    const poolType = new BN(args.poolType);
    const mint = new PublicKey(args.mint);

    const anchorProvider = await setAnchorProvider();
    const merkleTreeConfig = await getWalletConfig(anchorProvider);

    try {
      await merkleTreeConfig.registerSplPool(
        [...poolType.toArrayLike(Buffer, "be", 32)],
        mint,
      );
    } catch (e) {
      console.log(e);
    }

    loader.stop(false);
    this.log("SPL pool registered successfully \x1b[32m✔\x1b[0m");
  }
}

export default RegisterSplCommand;

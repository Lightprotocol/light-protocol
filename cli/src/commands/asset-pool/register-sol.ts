import { BN } from "@coral-xyz/anchor";
import { Args, Command } from "@oclif/core";
import { CustomLoader, getWalletConfig, setAnchorProvider } from "../../utils";

class RegisterSolCommand extends Command {
  static description = "Register SOL pool.";

  static examples = ["light asset-pool:register-sol"];

  static args = {
    poolType: Args.string({
      description: "Pool type to register the SOL pool in.",
      required: true,
    }),
  };

  async run() {
    const loader = new CustomLoader("Registering SOL pool");
    loader.start();

    const { args } = await this.parse(RegisterSolCommand);
    const poolType = new BN(args.poolType);

    const anchorProvider = await setAnchorProvider();
    const merkleTreeConfig = await getWalletConfig(anchorProvider);

    await merkleTreeConfig.registerSolPool([
      ...poolType.toArrayLike(Buffer, "be", 32),
    ]);

    loader.stop(false);
    this.log("SOL pool registered successfully \x1b[32m✔\x1b[0m");
  }
}

export default RegisterSolCommand;

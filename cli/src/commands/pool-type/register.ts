import { BN } from "@coral-xyz/anchor";
import { Args, Command } from "@oclif/core";
import { CustomLoader, getWalletConfig, setAnchorProvider } from "../../utils";

class PoolTypeRegister extends Command {
  static description = "Register pool type.";

  static examples = ["light pool-type:register 0"];

  static args = {
    poolType: Args.string({
      description: "Pool type to register.",
      required: true,
    }),
  };

  async run() {
    const loader = new CustomLoader("Registering pool type");
    loader.start();

    const { args } = await this.parse(PoolTypeRegister);
    const poolType = new BN(args.poolType);

    const anchorProvider = await setAnchorProvider();
    const merkleTreeConfig = await getWalletConfig(anchorProvider);

    await merkleTreeConfig.registerPoolType([
      ...poolType.toArrayLike(Buffer, "be", 32),
    ]);
    this.log("Pool type registered successfully \x1b[32m✔\x1b[0m");
    loader.stop(false);
  }
}

export default PoolTypeRegister;

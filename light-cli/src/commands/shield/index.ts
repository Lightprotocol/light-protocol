import { Command, Flags } from "@oclif/core";
import { User } from "@lightprotocol/zk.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getUser,
} from "../../utils/utils";

class ShieldCommand extends Command {
  static description = "Shield tokens for a user";

  static examples = [
    "$ light shield --token USDC --amountSpl 10",
    "$ light shield --token SOL --amountSpl 1 --recipient address",
  ];

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static flags = {
    token: Flags.string({
      description: "The token to shield",
      required: true,
    }),
    recipient: Flags.string({
      description: "The recipient shielded publickey",
    }),
    amountSpl: Flags.string({
      description: "The amount of token to shield (SPL)",
    }),
    amountSol: Flags.string({
      description: "The amount of token to shield (SOL)",
    }),
    minimumLamports: Flags.boolean({
      description:
        "Whether to use the minimum required lamports for the shield transaction",
      default: false,
    }),
    skipDecimalConversions: Flags.boolean({
      description: "Skip decimal conversions during shield",
      default: false,
    }),
  };

  async run() {
    const { flags } = await this.parse(ShieldCommand);

    const {
      token,
      recipient,
      amountSpl,
      amountSol,
      minimumLamports,
      skipDecimalConversions,
    } = flags;

    const loader = new CustomLoader("Performing shield operation...");

    loader.start();

    try {
      const user: User = await getUser();

      const response = await user.shield({
        token,
        recipient,
        publicAmountSpl: amountSpl ? amountSpl : 0,
        publicAmountSol: amountSol ? amountSol : 0,
        minimumLamports,
        skipDecimalConversions,
      });

      this.log(`\nToken shielded successfully: ${token}`);
      this.log(generateSolanaTransactionURL("tx", response.txHash, "custom"));
      loader.stop();
    } catch (error) {
      loader.stop();
      this.error(`\nShielding tokens failed: ${error}`);
    }
  }
}

ShieldCommand.strict = false;

export default ShieldCommand;

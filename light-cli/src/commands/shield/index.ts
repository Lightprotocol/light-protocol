import { Command, Flags } from "@oclif/core";
import { User } from "light-sdk";
import { generateSolanaTransactionURL, getLoader, getUser } from "../../utils";

class ShieldCommand extends Command {
  static description = "Shield tokens for a user";

  static examples = ["$ light shield --token USDC --amountSpl 10"];

  static flags = {
    token: Flags.string({
      description: "The token to shield",
      required: true,
    }),
    recipient: Flags.string({
      description: "The recipient address",
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

    const { loader, end } = getLoader("Performing shield...");

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

      this.log(`Successfully shielded: ${token}`);
      this.log(generateSolanaTransactionURL("tx", response.txHash, "custom"));
      end(loader);
    } catch (error) {
      end(loader);
      this.error(`Shielding tokens failed: ${error}`);
    }
  }
}

ShieldCommand.strict = false;

export default ShieldCommand;

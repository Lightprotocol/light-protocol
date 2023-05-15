import { Command, Flags } from "@oclif/core";
import { User } from "light-sdk";
import { getUser } from "../../utils";

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
    amountSpl: Flags.integer({
      description: "The amount of token to shield (SPL)",
    }),
    amountSol: Flags.integer({
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

    try {
      const user: User = await getUser();

      const response = await user.shield({
        token,
        recipient,
        publicAmountSpl: amountSpl,
        publicAmountSol: amountSol,
        minimumLamports,
        skipDecimalConversions,
      });

      console.log(response);

      const balance = await user.getBalance();

      console.log({ balance });

      this.log(`Successfully shielded: ${token}`);
    } catch (error) {
      this.error(`Shielding tokens failed: ${error}`);
    }
  }
}

ShieldCommand.strict = false;

export default ShieldCommand;

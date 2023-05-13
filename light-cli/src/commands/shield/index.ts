import { Command, Flags } from "@oclif/core";
import { getUser } from "../../utils"; // Assuming you have a file named 'utils.ts' exporting the 'connection' and 'provider' objects

class ShieldCommand extends Command {
  static description = "Shield tokens for a user";

  static examples = ["$ light shield --token USDC --publicAmountSpl 10"];

  static flags = {
    token: Flags.string({
      description: "The token to shield",
      required: true,
    }),
    recipient: Flags.string({
      description: "The recipient address",
    }),
    publicAmountSpl: Flags.integer({
      description: "The amount of token to shield (SPL)",
    }),
    publicAmountSol: Flags.integer({
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
      publicAmountSpl,
      publicAmountSol,
      minimumLamports,
      skipDecimalConversions,
    } = flags;

    try {
      const user = await getUser();

      await user.shield({
        token,
        recipient,
        publicAmountSpl,
        publicAmountSol,
        minimumLamports,
        skipDecimalConversions,
      });

      this.log(`Tokens successfully shielded for token: ${token}`);
    } catch (error) {
      this.error(`Shielding tokens failed: ${error}`);
    }
  }
}

ShieldCommand.strict = false;

export default ShieldCommand;

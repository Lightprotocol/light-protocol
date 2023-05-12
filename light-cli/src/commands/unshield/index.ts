import { Command, Flags } from "@oclif/core";
import { connection, provider } from "./utils"; // Assuming you have a file named 'utils.ts' exporting the 'connection' and 'provider' objects

class UnshieldCommand extends Command {
  static description = "Unshield tokens for a user";

  static examples: Command.Example[] = [
    "$ light-cli unshield --token ABC123 --publicAmountSpl 1000000",
  ];

  static flags = {
    token: Flags.string({
      description: "The token to unshield",
      required: true,
    }),
    recipientSpl: Flags.string({
      description: "The recipient SPL address",
    }),
    recipientSol: Flags.string({
      description: "The recipient SOL address",
    }),
    publicAmountSpl: Flags.integer({
      description: "The amount of token to unshield (SPL)",
    }),
    publicAmountSol: Flags.integer({
      description: "The amount of token to unshield (SOL)",
    }),
    minimumLamports: Flags.boolean({
      description:
        "Whether to use the minimum required lamports for the unshield transaction",
      default: false,
    }),
  };

  async run() {
    const { flags } = await this.parse(UnshieldCommand);

    const {
      token,
      recipientSpl,
      recipientSol,
      publicAmountSpl,
      publicAmountSol,
      minimumLamports,
    } = flags;

    try {
      const user = await User.init({ provider });

      await user.unshield({
        token,
        recipientSpl,
        recipientSol,
        publicAmountSpl,
        publicAmountSol,
        minimumLamports,
      });

      this.log(`Tokens successfully unshielded for token: ${token}`);
    } catch (error) {
      this.error(`Unshielding tokens failed: ${error.message}`);
    }
  }
}

UnshieldCommand.strict = false;

export default UnshieldCommand;

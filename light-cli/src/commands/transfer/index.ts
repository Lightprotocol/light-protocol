import { Args, Command, Flags } from "@oclif/core";
import { connection, provider } from "./utils"; // Assuming you have a file named 'utils.ts' exporting the 'connection' and 'provider' objects

class TransferCommand extends Command {
  static description = "Transfer tokens to a recipient";

  static examples = [
    "$ light-cli transfer --token ABC123 --amountSpl 1000000 <recipient>",
  ];

  static flags = {
    token: Flags.string({
      description: "The token to transfer",
      required: true,
    }),
    amountSpl: Flags.integer({
      description: "The amount of token to transfer (SPL)",
    }),
    amountSol: Flags.integer({
      description: "The amount of token to transfer (SOL)",
    }),
  };

  static args = {
    recipient: Args.string({
      name: "recipient",
      description: "The recipient address",
      required: true,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(TransferCommand);

    const { recipient } = args;
    const { token, amountSpl, amountSol } = flags;

    try {
      const user = await User.init({ provider });

      await user.transfer({
        token,
        amountSpl,
        amountSol,
        recipient,
      });

      this.log(`Tokens successfully transferred to recipient: ${recipient}`);
    } catch (error) {
      this.error(`Transfer failed: ${error.message}`);
    }
  }
}

TransferCommand.strict = false;

export default TransferCommand;

import { Args, Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getConfirmOptions,
  getUser,
} from "../../utils/utils";
import { confirmOptionsFlags, standardFlags } from "../../utils";

class TransferCommand extends Command {
  static summary = "Transfer compressed funds between light users.";

  static examples = [
    "$ light transfer 1.8 <COMPRESSED_RECIPIENT_ADDRESS>",
    "$ light transfer 10 <COMPRESSED_RECIPIENT_ADDRESS> -t USDC",
  ];

  static flags = {
    ...standardFlags,
    ...confirmOptionsFlags,
    token: Flags.string({
      char: "t",
      description: "The SPL token symbol.",
      parse: async (token: string) => token.toUpperCase(),
      default: "SOL",
    }),
  };

  static args = {
    amount: Args.string({
      name: "AMOUNT",
      description: "The token amount to tranfer.",
      required: true,
    }),
    recipient: Args.string({
      name: "COMPRESSED_RECIPIENT_ADDRESS",
      description: "The recipient compressed/encryption public key.",
      required: true,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(TransferCommand);
    const { recipient, amount } = args;
    const { token } = flags;

    const loader = new CustomLoader(
      `Performing compressed ${token} transfer...\n`,
    );
    loader.start();

    try {
      let amountSol, amountSpl;
      if (token === "SOL") amountSol = amount;
      else amountSpl = amount;

      const user = await getUser({
        skipFetchBalance: flags["skipFetchBalance"],
        localTestRpc: flags["localTestRpc"],
      });
      const response = await user.transfer({
        token,
        amountSpl,
        amountSol,
        recipient,
        confirmOptions: getConfirmOptions(flags),
      });

      this.log(
        generateSolanaTransactionURL(
          "tx",
          `${response.txHash.signatures}`,
          "custom",
        ),
      );
      this.log(
        `\nSuccessfully transferred ${
          token === "SOL" ? amountSol : amountSpl
        } ${token}`,
        "\x1b[32m✔\x1b[0m",
      );
      loader.stop();
    } catch (error) {
      this.error(`Failed to transfer ${token}!\n${error}`);
    }
  }
}

export default TransferCommand;

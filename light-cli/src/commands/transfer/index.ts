import { Args, Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getConfirmOptions,
  getUser,
} from "../../utils/utils";
import { confirmOptionsFlags, standardFlags } from "../../utils";

class TransferCommand extends Command {
  static summary = "Transfer shielded funds between light users";

  static examples = [
    "$ light transfer 1.8 <SHIELDED_RECIPIENT_ADDRESS>",
    "$ light transfer 10 <SHIELDED_RECIPIENT_ADDRESS> -t USDC",
  ];

  static flags = {
    ...standardFlags,
    ...confirmOptionsFlags,
    token: Flags.string({
      char: "t",
      description: "The SPL token symbol",
      parse: async (token) => token.toUpperCase(),
      default: "SOL",
    }),
  };

  static args = {
    amount: Args.string({
      name: "AMOUNT",
      description: "The token amount to tranfer",
      required: true,
    }),
    recipient: Args.string({
      name: "SHIELDED_RECIPIENT_ADDRESS",
      description: "The recipient shielded/encryption publickey",
      required: true,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(TransferCommand);
    const { recipient, amount } = args;
    const { token } = flags;

    const loader = new CustomLoader(
      `Performing shielded ${token} transfer...\n`
    );
    loader.start();

    try {
      let amountSol, amountSpl;
      if (token === "SOL") amountSol = amount;
      else amountSpl = amount;

      const user = await getUser(flags["skipBalanceFetch"]);
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
          "custom"
        )
      );
      this.log(
        `\nSuccessfully transferred ${
          token.toLowerCase() === "sol" ? amountSol : amountSpl
        } ${token}`,
        "\x1b[32m✔\x1b[0m"
      );
      loader.stop();
    } catch (error) {
      this.error(`Failed to transfer ${token}!\n${error}`);
    }
  }
}

export default TransferCommand;

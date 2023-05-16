import { Args, Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getUser,
  readWalletFromFile,
} from "../../utils/utils";
import { User } from "@lightprotocol/zk.js";

class TransferCommand extends Command {
  static description = "Transfer tokens to a recipient";

  static examples = [
    "$ light transfer --token ABC123 --amountSpl 1000000 <recipient>",
  ];

  static flags = {
    token: Flags.string({
      description: "The token to transfer",
      required: true,
    }),
    amountSpl: Flags.string({
      description: "The amount of token to transfer (SPL)",
    }),
    amountSol: Flags.string({
      description: "The amount of token to transfer (SOL)",
    }),
  };

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static args = {
    recipient: Args.string({
      name: "recipient",
      description: "The recipient shielded/encryption publickey",
      required: true,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(TransferCommand);

    const { recipient } = args;
    const { token, amountSpl, amountSol } = flags;

    const loader = new CustomLoader("Performing token transfer...");

    loader.start();

    try {
      await readWalletFromFile();

      const user: User = await getUser();

      const response = await user.transfer({
        token,
        amountSpl,
        amountSol,
        recipient,
      });

      this.log(
        `Successfully transferred ${
          token.toLowerCase() === "sol" ? amountSol : amountSpl
        } ${token}`
      );
      this.log(generateSolanaTransactionURL("tx", response.txHash, "custom"));
      loader.stop();
    } catch (error) {
      loader.stop();

      this.error(`\nToken transfer failed: ${error}`);
    }
  }
}

TransferCommand.strict = false;

export default TransferCommand;

import { Args, Command, Flags } from "@oclif/core";
import { User } from "@lightprotocol/zk.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getUser,
  readWalletFromFile,
} from "../../utils/utils";

class TransferCommand extends Command {
  static summary = "Transfer shielded funds between light users";

  static examples = [
    "$ light transfer 1.8 <SHIELDED_RECIPIENT_ADDRESS>",
    "$ light transfer 10 <SHIELDED_RECIPIENT_ADDRESS> -t USDC"
  ];

  static flags = {
    token: Flags.string({
      char: "t",
      description: "The SPL token symbol",
      default: "SOL"
    }),
  };

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static args = {
    amount: Args.string({
      name: "AMOUNT",
      description: "The token amount to tranfer",
      required: true,
    }),
    psp_recipient_address: Args.string({
      name: "SHIELDED_RECIPIENT_ADDRESS",
      description: "The recipient shielded/encryption publickey",
      required: true,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(TransferCommand);

    const { psp_recipient_address, amount } = args;
    const { token } = flags;

    const loader = new CustomLoader(`Performing shielded ${token} transfer...\n`);

    loader.start();

    try {
      await readWalletFromFile();

      const user: User = await getUser();

      let amountSol, amountSpl;
      if (token === "SOL") amountSol = amount;
      else amountSpl = amount;

      const originalConsoleLog = console.log;      
      console.log = function(...args) {
        if (args[0] !== 'shuffle disabled') {
          originalConsoleLog.apply(console, args);
        }
      };
      const response = await user.transfer({
        token,
        amountSpl,
        amountSol,
        recipient: psp_recipient_address,
      });

      this.log(generateSolanaTransactionURL("tx", response.txHash, "custom"));

      this.log(
        `\nSuccessfully transferred ${
          token.toLowerCase() === "sol" ? amountSol : amountSpl
        } ${token}`,
        "\x1b[32m✔\x1b[0m"
      );
      loader.stop();
    } catch (error) {
      this.warn(error as Error);
      loader.stop();
      this.error(`\nToken transfer failed: ${error}`);
    }
  }

}

TransferCommand.strict = false;

export default TransferCommand;

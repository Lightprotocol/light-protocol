import { Command, Flags, Args } from "@oclif/core";
import { User } from "@lightprotocol/zk.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getUser,
} from "../../utils/utils";

class ShieldCommand extends Command {
  static summary = "Shield SOL for a user";
  static usage = "shield:sol <AMOUNT> [FLAGS]"
  static examples = [
    "$ light shield:sol 1.3 --recipient <SHIELDED_RECIPIENT_ADDRESS> ",
    "$ light shield:sol 12345678 -s"
  ];
  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static flags = {
    "recipient": Flags.string({
      char: "r",
      description: "The recipient shielded/encryption publickey. If not set, the operation will shield to self.",
      required: false
    }),
    'skip-minimum-lamports': Flags.boolean({
      description:
        "Whether to use the minimum required lamports for the shield transaction",
      default: false,
    }),
    'skip-decimal-conversions': Flags.boolean({
      char: "d",
      description: "Skip decimal conversions during shield",
      default: false,
    }),
  };

  static args = {
    amount: Args.string({
      name: "AMOUNT",
      description: "The SOL amount to shield",
      required: true,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(ShieldCommand);
    let amountSol = args.amount;

    const recipient = flags["recipient"];
    const minimumLamports = flags["minimum-lamports"];
    const skipDecimalConversions = flags["skip-decimal-conversions"];

    const loader = new CustomLoader("Performing shield operation...\n");

    loader.start();

    try {

      const originalConsoleLog = console.log;      
      console.log = function(...args) {
        if (args[0] !== 'shuffle disabled') {
          originalConsoleLog.apply(console, args);
        }
      };
      
      const user: User = await getUser();
      const response = await user.shield({
        token: "SOL",
        recipient,
        publicAmountSol: amountSol,
        minimumLamports,
        skipDecimalConversions,
      });

      this.log(generateSolanaTransactionURL("tx", response.txHash, "custom"));
      let amount = skipDecimalConversions ? Number(amountSol) / 1_000_000_000 : amountSol;
      this.log(
        `\nSuccessfully shielded ${amount} SOL`,
        "\x1b[32m✔\x1b[0m"
      );
      loader.stop();
    } catch (error) {
      this.warn(error as Error);
      loader.stop();
      this.error(`\nShielding tokens failed: ${error}`);
    }
  }
}

ShieldCommand.strict = false;

export default ShieldCommand;

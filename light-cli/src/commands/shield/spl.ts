import { Command, Flags, Args } from "@oclif/core";
import { User } from "@lightprotocol/zk.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getUser,
} from "../../utils/utils";

class ShieldCommand extends Command {
  static summary = "Shield SPL tokens for a user";

  static examples = [
    "$ light shield:spl 10 USDC",
    "$ light shield:spl 13 USDT --recipient <SHIELDED_RECIPIENT_ADDRESS>",
  ];

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static flags = {
    'recipient': Flags.string({
      char: "r",
      description: "The recipient shielded/encryption publickey. If not set, the operation will shield to self.",
      required: false
    }),
    'minimum-lamports': Flags.boolean({
      description:
        "Whether to use the minimum required lamports for the shield transaction",
      default: false,
    }),
    'skip-decimal-conversions': Flags.boolean({
      description: "Skip decimal conversions during shield",
      default: false,
    }),
  };

  static args = {
    amount: Args.string({
      name: "AMOUNT",
      description: "The SPL token amount to shield",
      required: true,
    }),
    token: Args.string({
      name: "TOKEN",
      description: "The SPL token symbol",
      parse: async (token) => token.toUpperCase(), 
      required: true,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(ShieldCommand);
    
    const amountSpl = args.amount;
    const token = args.token;
    
    const recipient = flags['recipient'];
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
        token,
        recipient,
        publicAmountSpl: amountSpl,
        publicAmountSol: 0,
        minimumLamports,
        skipDecimalConversions,
      });

      this.log(generateSolanaTransactionURL("tx", response.txHash, "custom"));
      this.log(
        `\nSuccessfully shielded ${amountSpl} ${token}`,
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

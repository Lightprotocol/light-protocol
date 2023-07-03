import { Command, Flags, Args } from "@oclif/core";
import { TOKEN_REGISTRY, ADMIN_AUTH_KEYPAIR } from "@lightprotocol/zk.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getUser,
} from "../../utils/utils";

class ShieldSplCommand extends Command {
  static summary = "Shield SPL tokens for a user";

  static examples = [
    "$ light shield:spl 10 USDC",
    "$ light shield:spl 13 USDT --recipient <SHIELDED_RECIPIENT_ADDRESS>",
  ];

  static flags = {
    'recipient': Flags.string({
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
      char: 'd',
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
    const { args, flags } = await this.parse(ShieldSplCommand);

    const amountSpl = args.amount;
    const token = args.token;
    
    const recipient = flags['recipient'];
    const minimumLamports = flags["minimum-lamports"];
    const skipDecimalConversions = flags["skip-decimal-conversions"];
    
    const loader = new CustomLoader("Performing shield operation...\n");
    loader.start();

    try {
      const decimals = TOKEN_REGISTRY.get(token)?.decimals.toNumber();
      
      const user = await getUser(ADMIN_AUTH_KEYPAIR);
      const response = await user.shield({
        token,
        recipient,
        publicAmountSpl: amountSpl,
        minimumLamports,
        skipDecimalConversions,
      });

      this.log(generateSolanaTransactionURL("tx", `${response.txHash.signatures}`, "custom"));
      let amount = skipDecimalConversions ? Number(amountSpl) / decimals! : amountSpl;

      this.log(
        `\nSuccessfully shielded ${amount} ${token}`,
        "\x1b[32m✔\x1b[0m"
      );
      loader.stop();
    } catch (error) {
      this.error(`\nFailed to shield ${token}\n${error}`);
    }
  }
}

export default ShieldSplCommand;

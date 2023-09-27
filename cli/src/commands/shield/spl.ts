import { Command, Args } from "@oclif/core";
import { TOKEN_REGISTRY } from "@lightprotocol/zk.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getConfirmOptions,
  getUser,
  confirmOptionsFlags,
  standardFlags
} from "../../utils";
import { shieldFlags, shieldSolFlags } from ".";

class ShieldSplCommand extends Command {
  static summary = "Shield SPL tokens for a user.";

  static examples = [
    "$ light shield:SPL 10 USDC",
    "$ light shield:SPL 13 USDT --recipient <SHIELDED_RECIPIENT_ADDRESS>",
  ];

  static flags = {
    ...standardFlags,
    ...standardFlags,
    ...shieldSolFlags,
    ...confirmOptionsFlags,
    "skip-minimum-lamports": shieldFlags["skip-minimum-lamports"],
  };

  static args = {
    amount: Args.string({
      name: "AMOUNT",
      description: "The SPL token amount to shield.",
      required: true,
    }),
    token: Args.string({
      name: "TOKEN",
      description: "The SPL token symbol.",
      parse: async (token) => token.toUpperCase(),
      required: true,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(ShieldSplCommand);

    const amountSpl = args.amount;
    const token = args.token;

    const recipient = flags["recipient"];
    const minimumLamports = flags["minimum-lamports"];
    const skipDecimalConversions = flags["skip-decimal-conversions"];

    const loader = new CustomLoader("Performing shield operation...\n");
    loader.start();

    try {
      const decimals = TOKEN_REGISTRY.get(token)?.decimals.toNumber();

      const user = await getUser({
        skipFetchBalance: flags["skipFetchBalance"],
        localTestRelayer: flags["localTestRelayer"],
      });
      const response = await user.shield({
        token,
        recipient,
        publicAmountSpl: amountSpl,
        minimumLamports,
        skipDecimalConversions,
        confirmOptions: getConfirmOptions(flags),
      });

      this.log(
        generateSolanaTransactionURL(
          "tx",
          `${response.txHash.signatures}`,
          "custom"
        )
      );
      const amount = skipDecimalConversions
        ? Number(amountSpl) / decimals!
        : amountSpl;

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

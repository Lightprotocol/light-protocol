import { Command, Args } from "@oclif/core";
import { TOKEN_REGISTRY } from "@lightprotocol/zk.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getConfirmOptions,
  getUser,
  confirmOptionsFlags,
  standardFlags,
} from "../../utils";
import { compressFlags, compressSolFlags } from ".";

class CompressSplCommand extends Command {
  static summary = "Compress SPL tokens for a user.";

  static examples = [
    "$ light compress:SPL 10 USDC",
    "$ light compress:SPL 13 USDT --recipient <COMPRESSED_RECIPIENT_ADDRESS>",
  ];

  static flags = {
    ...standardFlags,
    ...standardFlags,
    ...compressSolFlags,
    ...confirmOptionsFlags,
    "skip-minimum-lamports": compressFlags["skip-minimum-lamports"],
  };

  static args = {
    amount: Args.string({
      name: "AMOUNT",
      description: "The SPL token amount to compress.",
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
    const { args, flags } = await this.parse(CompressSplCommand);

    const amountSpl = args.amount;
    const token = args.token;

    const recipient = flags["recipient"];
    const minimumLamports = flags["minimum-lamports"];
    const skipDecimalConversions = flags["skip-decimal-conversions"];

    const loader = new CustomLoader("Performing compress operation...\n");
    loader.start();

    try {
      const decimals = TOKEN_REGISTRY.get(token)?.decimals.toNumber();

      const user = await getUser({
        skipFetchBalance: flags["skipFetchBalance"],
        localTestRpc: flags["localTestRpc"],
      });
      const response = await user.compress({
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
          "custom",
        ),
      );
      const amount = skipDecimalConversions
        ? Number(amountSpl) / decimals!
        : amountSpl;

      this.log(
        `\nSuccessfully compressed ${amount} ${token}`,
        "\x1b[32m✔\x1b[0m",
      );
      loader.stop();
    } catch (error) {
      this.error(`\nFailed to compress ${token}\n${error}`);
    }
  }
}

export default CompressSplCommand;

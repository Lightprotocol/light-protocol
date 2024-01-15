import { Command, Flags } from "@oclif/core";
import { User } from "@lightprotocol/zk.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getConfirmOptions,
  getUser,
} from "../../utils/utils";
import { confirmOptionsFlags, standardFlags } from "../../utils";

export const compressSolFlags = {
  recipient: Flags.string({
    char: "r",
    description:
      "The recipient compressed/encryption publickey. If not set, the operation will compress to self.",
    required: false,
  }),
  "skip-decimal-conversions": Flags.boolean({
    char: "d",
    description: "Skip decimal conversions during compress.",
    default: false,
  }),
};

export const compressFlags = {
  token: Flags.string({
    char: "t",
    description: "The SPL token symbol.",
    parse: async (token: string) => token.toUpperCase(),
    default: "SOL",
  }),
  "amount-spl": Flags.string({
    char: "p",
    description: "The SPL token amount to compress.",
    relationships: [
      {
        type: "some",
        flags: [
          {
            name: "token",
            when: async (flags: any) => flags["token"] !== "SOL",
          },
        ],
      },
    ],
  }),
  "amount-sol": Flags.string({
    char: "l",
    description: "The SOL amount to compress.",
  }),
  "skip-minimum-lamports": Flags.boolean({
    description:
      "Whether to use the minimum required lamports for the compress transaction.",
    default: false,
  }),
};

class CompressCommand extends Command {
  static summary = "Compress tokens for a user";
  static examples = [
    "$ light compress --amount-sol 1.3 --recipient <COMPRESSED_RECIPIENT_ADDRESS>",
    "$ light compress --amount-spl 15 -t USDC",
    "$ light compress --amount-sol 1 --amount-spl 22 -t USDC",
  ];

  static flags = {
    ...standardFlags,
    ...compressFlags,
    ...compressSolFlags,
    ...confirmOptionsFlags,
  };

  async run() {
    const { flags } = await this.parse(CompressCommand);
    const token = flags["token"];
    const amountSol = flags["amount-sol"];
    const amountSpl = flags["amount-spl"];
    const recipient = flags["recipient"];
    const minimumLamports = flags["skip-minimum-lamports"];
    const skipDecimalConversions = flags["skip-decimal-conversions"];
    const skipFetchBalance = flags["skipFetchBalance"];

    const loader = new CustomLoader("Performing compress operation...\n");
    loader.start();

    try {
      const user: User = await getUser({
        skipFetchBalance,
        localTestRpc: flags["localTestRpc"],
      });
      const response = await user.compress({
        token,
        recipient,
        publicAmountSpl: amountSpl ? amountSpl : 0,
        publicAmountSol: amountSol ? amountSol : 0,
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

      if (!amountSol || !amountSpl) {
        this.log(
          `\nSuccessfully compressed ${
            token === "SOL" ? amountSol : amountSpl
          } ${token}`,
          "\x1b[32m✔\x1b[0m",
        );
      } else {
        this.log(
          `\nSuccessfully compressed ${amountSol} SOL & ${amountSpl} ${token}`,
          "\x1b[32m✔\x1b[0m",
        );
      }
      loader.stop();
    } catch (error) {
      this.logToStderr(`${error}\n`);
      this.exit(2);
      loader.stop();
    }
  }
}

export default CompressCommand;

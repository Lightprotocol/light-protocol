import { Command, Flags } from "@oclif/core";
import { User } from "@lightprotocol/zk.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getConfirmOptions,
  getUser,
} from "../../utils/utils";
import { confirmOptionsFlags, standardFlags } from "../../utils";

export const shieldSolFlags = {
  recipient: Flags.string({
    char: "r",
    description:
      "The recipient shielded/encryption publickey. If not set, the operation will shield to self.",
    required: false,
  }),
  "skip-decimal-conversions": Flags.boolean({
    char: "d",
    description: "Skip decimal conversions during shield.",
    default: false,
  }),
};

export const shieldFlags = {
  token: Flags.string({
    char: "t",
    description: "The SPL token symbol.",
    parse: async (token: string) => token.toUpperCase(),
    default: "SOL",
  }),
  "amount-spl": Flags.string({
    char: "p",
    description: "The SPL token amount to shield.",
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
    description: "The SOL amount to shield.",
  }),
  "skip-minimum-lamports": Flags.boolean({
    description:
      "Whether to use the minimum required lamports for the shield transaction.",
    default: false,
  }),
};

class ShieldCommand extends Command {
  static summary = "Shield tokens for a user";
  static examples = [
    "$ light shield --amount-sol 1.3 --recipient <SHIELDED_RECIPIENT_ADDRESS>",
    "$ light shield --amount-spl 15 -t USDC",
    "$ light shield --amount-sol 1 --amount-spl 22 -t USDC",
  ];

  static flags = {
    ...standardFlags,
    ...shieldFlags,
    ...shieldSolFlags,
    ...confirmOptionsFlags,
  };

  async run() {
    const { flags } = await this.parse(ShieldCommand);
    const token = flags["token"];
    const amountSol = flags["amount-sol"];
    const amountSpl = flags["amount-spl"];
    const recipient = flags["recipient"];
    const minimumLamports = flags["skip-minimum-lamports"];
    const skipDecimalConversions = flags["skip-decimal-conversions"];
    const skipFetchBalance = flags["skipFetchBalance"];

    const loader = new CustomLoader("Performing shield operation...\n");
    loader.start();

    try {
      const user: User = await getUser({
        skipFetchBalance,
        localTestRelayer: flags["localTestRelayer"],
      });
      const response = await user.shield({
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
          "custom"
        )
      );

      if (!amountSol || !amountSpl) {
        this.log(
          `\nSuccessfully shielded ${
            token === "SOL" ? amountSol : amountSpl
          } ${token}`,
          "\x1b[32m✔\x1b[0m"
        );
      } else {
        this.log(
          `\nSuccessfully shielded ${amountSol} SOL & ${amountSpl} ${token}`,
          "\x1b[32m✔\x1b[0m"
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

export default ShieldCommand;

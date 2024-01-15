import { Command, Flags } from "@oclif/core";
import { PublicKey } from "@solana/web3.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getConfirmOptions,
  getUser,
} from "../../utils/utils";
import { confirmOptionsFlags, standardFlags } from "../../utils";
class UnshieldCommand extends Command {
  static summary = "Unshield tokens for a user.";
  static examples = [
    "$ light unshield --amount-SOL 2.4 --recipient <RECIPIENT_ADDRESS>",
    "$ light unshield --token USDC --amount-SPL 22 --recipient <RECIPIENT_ADDRESS>",
    "$ light unshield --amount-SOL 1.2 --amount-SPL 12 --token USDC --recipient <RECIPIENT_ADDRESS>",
  ];

  static flags = {
    ...standardFlags,
    ...confirmOptionsFlags,
    token: Flags.string({
      char: "t",
      description: "The token to unshield.",
      default: "SOL",
      parse: async (token: string) => token.toUpperCase(),
    }),
    recipient: Flags.string({
      char: "r",
      description: "The recipient SOL account address.",
    }),
    "amount-spl": Flags.string({
      description: "The SPL token amount to unshield.",
      dependsOn: ["token"],
    }),
    "amount-sol": Flags.string({
      description: "The SOL amount to unshield.",
    }),
    "skip-minimum-lamports": Flags.boolean({
      description:
        "Whether to use the minimum required lamports for the unshield transaction.",
      default: false,
    }),
  };

  async run() {
    const { flags } = await this.parse(UnshieldCommand);
    const token = flags["token"];
    const amountSol = flags["amount-sol"];
    const recipient = flags["recipient"];
    const amountSpl = flags["amount-spl"];
    const minimumLamports = flags["minimum-lamports"];

    const loader = new CustomLoader("Performing token unshield...");
    loader.start();

    try {
      const user = await getUser({
        skipFetchBalance: flags["skipFetchBalance"],
        localTestRpc: flags["localTestRpc"],
      });
      const response = await user.unshield({
        token,
        recipient: recipient ? new PublicKey(recipient) : undefined,
        publicAmountSpl: amountSpl ? Number(amountSpl) : undefined,
        publicAmountSol: amountSol ? Number(amountSol) : undefined,
        minimumLamports,
        confirmOptions: getConfirmOptions(flags),
      });

      if (!amountSol || !amountSpl) {
        this.log(
          `\nSuccessfully decompressed ${
            token === "SOL" ? amountSol : amountSpl
          } ${token}`,
          "\x1b[32m✔\x1b[0m",
        );
      } else {
        this.log(
          `\nSuccessfully decompressed ${amountSol} SOL & ${amountSpl} ${token}`,
          "\x1b[32m✔\x1b[0m",
        );
      }
      this.log(
        generateSolanaTransactionURL(
          "tx",
          `${response.txHash.signatures}`,
          "custom",
        ),
      );
      loader.stop();
    } catch (error) {
      this.error(`Failed to unshield ${token}!\n${error}`);
    }
  }
}

export default UnshieldCommand;

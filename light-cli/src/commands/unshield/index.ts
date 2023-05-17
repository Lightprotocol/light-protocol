import { Command, Flags } from "@oclif/core";
import { generateSolanaTransactionURL, getUser } from "../../utils";
import { PublicKey } from "@solana/web3.js";
import { User } from "light-sdk";

class UnshieldCommand extends Command {
  static description = "Unshield tokens for a user";

  static examples: Command.Example[] = [
    "$ light unshield --token USDC --amountSpl 1000000 --recipienSpl <address>",
  ];

  static flags = {
    token: Flags.string({
      description: "The token to unshield",
      required: true,
    }),
    recipientSpl: Flags.string({
      description: "The recipient SPL address",
    }),
    recipientSol: Flags.string({
      description: "The recipient SOL address",
    }),
    amountSpl: Flags.string({
      description: "The amount of token to unshield (SPL)",
    }),
    amountSol: Flags.string({
      description: "The amount of token to unshield (SOL)",
    }),
    minimumLamports: Flags.boolean({
      description:
        "Whether to use the minimum required lamports for the unshield transaction",
      default: false,
    }),
  };

  async run() {
    const { flags } = await this.parse(UnshieldCommand);

    const {
      token,
      recipientSpl,
      recipientSol,
      amountSpl,
      amountSol,
      minimumLamports,
    } = flags;

    try {
      const user: User = await getUser();

      const response = await user.unshield({
        token,
        recipientSpl: recipientSpl
          ? new PublicKey(recipientSpl)
          : PublicKey.default,
        recipientSol: recipientSol
          ? new PublicKey(recipientSol)
          : PublicKey.default,
        publicAmountSpl: amountSpl,
        publicAmountSol: amountSol,
        minimumLamports,
      });

      this.log(`Successfully unshielded ${token}`);
      this.log(generateSolanaTransactionURL("tx", response.txHash, "custom"));
    } catch (error) {
      this.error(`Unshielding tokens failed: ${error}`);
    }
  }
}

UnshieldCommand.strict = false;

export default UnshieldCommand;

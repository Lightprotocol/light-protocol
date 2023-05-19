import { Command, Flags } from "@oclif/core";
import { generateSolanaTransactionURL, getLoader, getUser } from "../../utils";
import { PublicKey } from "@solana/web3.js";
import { User } from "light-sdk";

class UnshieldCommand extends Command {
  static description = "Unshield tokens for a user";

  static examples = [
    "$ light unshield --token USDC --amountSpl 1000000 --recipientSpl <address>",
  ];

  static flags = {
    token: Flags.string({
      description: "The token to unshield",
      required: true,
    }),
    recipientSpl: Flags.string({
      description: "The SPL recipient shielded publickey",
    }),
    recipientSol: Flags.string({
      description: "The SOL recipient shielded publickey",
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

    const { loader, end } = getLoader("Performing token unshield...");

    try {
      const user: User = await getUser();

      const response = await user.unshield({
        token,
        recipientSpl: recipientSpl ? new PublicKey(recipientSpl) : undefined,
        recipientSol: recipientSol ? new PublicKey(recipientSol) : undefined,
        publicAmountSpl: amountSpl ? Number(amountSpl) : undefined,
        publicAmountSol: amountSol ? Number(amountSol) : undefined,
        minimumLamports,
      });

      this.log(`Tokens successfully unshielded: ${token}`);
      this.log(generateSolanaTransactionURL("tx", response.txHash, "custom"));
      end(loader);
    } catch (error) {
      end(loader);
      this.error(`Token unshield failed: ${error}`);
    }
  }
}

UnshieldCommand.strict = false;

export default UnshieldCommand;

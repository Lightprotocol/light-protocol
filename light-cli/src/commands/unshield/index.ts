import { Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getUser,
} from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";
import { User } from "@lightprotocol/zk.js";

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

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

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

    const loader = new CustomLoader("Performing token unshield...");

    loader.start();

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

      this.log(`\nTokens successfully unshielded: ${token}`);
      this.log(generateSolanaTransactionURL("tx", response.txHash, "custom"));
      loader.stop();
    } catch (error) {
      loader.stop();

      this.error(`\nToken unshield failed: ${error}`);
    }
  }
}

UnshieldCommand.strict = false;

export default UnshieldCommand;

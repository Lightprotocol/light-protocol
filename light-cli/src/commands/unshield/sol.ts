import { Command, Flags, Args } from "@oclif/core";
import { PublicKey } from "@solana/web3.js";
import { User } from "@lightprotocol/zk.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getUser,
} from "../../utils/utils";

class UnshieldCommand extends Command {
  static summary = "Unshield SOL for a user";
  static usage = "unshield:sol <AMOUNT> <RECIPIENT_ADDRESS> [FLAGS]";
  static examples = [
    "$ light unshield:sol 5 <RECIPIENT_ADDRESS>",
  ];

  static flags = {
    'minimum-lamports': Flags.boolean({
      char: "m",
      description:
        "Whether to use the minimum required lamports for the unshield transaction",
      default: false,
    }),
  };

  static args = {
    amount: Args.string({
      name: "AMOUNT",
      description: "The SOL amount to unshield",
      required: true,
    }),
    recipient_address: Args.string({
      name: "RECIPIENT_ADDRESS",
      description: "The SOL account address of recipient.",
      required: true,
    }),
  };

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  async run() {
    const { args, flags } = await this.parse(UnshieldCommand);
    const amountSol = args.amount;
    const recipientSol = args.recipient_address;
    const minimumLamports = flags["minimum-lamports"];

    const loader = new CustomLoader("Performing token unshield...\n");
    loader.start();

    try {
      // ignore undesired logs
      const originalConsoleLog = console.log;      
      console.log = function(...args) {
        if (args[0] !== 'shuffle disabled') {
          originalConsoleLog.apply(console, args);
        }
      };

      const user: User = await getUser();
      const response = await user.unshield({
        token: "SOL",
        recipientSol: new PublicKey(recipientSol),
        publicAmountSol: amountSol,
        minimumLamports,
      });

      this.log(generateSolanaTransactionURL("tx", response.txHash, "custom"));
      this.log(
        `\nSuccessfully unshielded ${amountSol} SOL`,
        "\x1b[32m✔\x1b[0m"
      );
      loader.stop();
    } catch (error) {
      this.warn(error as Error);
      loader.stop();
      this.error(`\nToken unshield failed: ${error}`);
    }
  }
}

UnshieldCommand.strict = false;

export default UnshieldCommand;

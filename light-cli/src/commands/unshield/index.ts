import { Command, Flags } from "@oclif/core";
import { PublicKey } from "@solana/web3.js";
import { User } from "@lightprotocol/zk.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getUser,
} from "../../utils/utils";
class UnshieldCommand extends Command {
  static summary = "Unshield tokens for a user";
  static examples = [
    "$ light unshield --amount-sol 2.4 --recipient-sol <RECIPIENT_ADDRESS>",
    "$ light unshield --token USDC --amount-spl 22 --recipient-spl <RECIPIENT_ADDRESS>",
    "$ light unshield --amount-sol 1.2 --recipient-sol <RECIPIENT_ADDRESS> --amount-spl 12 --token USDC --recipient-spl <RECIPIENT_ADDRESS>"
  ];

  static flags = {
    'token': Flags.string({
      char: "t",
      description: "The token to unshield",
      default: "SOL",
      parse: async (token) => token.toUpperCase(), 
      required: false,
    }),
    'recipient': Flags.string({
      description: "The recipient SOL account address",
    }),
    'amount-spl': Flags.string({
      description: "The SPL token amount to unshield",
      dependsOn: ['token']
    }),
    'amount-sol': Flags.string({
      description: "The SOL amount to unshield",
    }),
    'skip-minimum-lamports': Flags.boolean({
      description:
        "Whether to use the minimum required lamports for the unshield transaction",
      default: false,
    }),
  };

  async run() {
    const { flags } = await this.parse(UnshieldCommand);
    const token = flags['token'];
    const amountSol = flags['amount-sol'];
    const recipient = flags['recipient'];
    const amountSpl = flags['amount-spl'];
    const minimumLamports = flags["minimum-lamports"];

    const loader = new CustomLoader("Performing token unshield...");
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
        token,
        recipient: recipient ? new PublicKey(recipient) : undefined,
        publicAmountSpl: amountSpl ? Number(amountSpl) : undefined,
        publicAmountSol: amountSol ? Number(amountSol) : undefined,
        minimumLamports,
      });

      if (!amountSol || !amountSpl ) {
        this.log(
          `\nSuccessfully unshielded ${
            token.toLowerCase() === "sol" ? amountSol : amountSpl
          } ${token}`,
          "\x1b[32m✔\x1b[0m"
        );
      }
      else {
        this.log(
          `\nSuccessfully unshielded ${amountSol} SOL & ${amountSpl} ${token}`,
          "\x1b[32m✔\x1b[0m"
        );
      }
      this.log(generateSolanaTransactionURL("tx", `${response.txHash.signatures}`, "custom"));
      loader.stop();
    } catch (error) {
      loader.stop();
      this.error(`Failed to unshield ${token}!\n${error}`);
    }
  }
}

export default UnshieldCommand;

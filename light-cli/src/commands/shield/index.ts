import { Command, Flags } from "@oclif/core";
import { User } from "@lightprotocol/zk.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getUser,
} from "../../utils/utils";
class ShieldCommand extends Command {
  static summary = "Shield tokens for a user";
  static examples = [
    "$ light shield --amount-sol 1.3 --recipient <SHIELDED_RECIPIENT_ADDRESS>",
    "$ light shield --amount-spl 15 -t USDC",
    "$ light shield --amount-sol 1 --amount-spl 22 -t USDC"
  ];

  static flags = {
    'token': Flags.string({
      char: "t",
      description: "The SPL token symbol",
      parse: async (token) => token.toUpperCase(), 
      default: "SOL",
    }),
    'recipient': Flags.string({
      char: "r",
      description: "The recipient shielded/encryption publickey. If not set, the operation will shield to self.",
      required: false
    }),
    'amount-spl': Flags.string({
      char: "p",
      description: "The SPL token amount to shield",
      relationships: [
        {type: 'some', flags: [
          {name: 'token', when: async (flags: any) => flags['token'] !== 'SOL'}
        ]}  
      ]
    }),
    'amount-sol': Flags.string({
      char: "l",
      description: "The SOL amount to shield",
    }),
    'skip-minimum-lamports': Flags.boolean({
      description:
        "Whether to use the minimum required lamports for the shield transaction",
      default: false,
    }),
    'skip-decimal-conversions': Flags.boolean({
      char: "d",
      description: "Skip decimal conversions during shield",
      default: false,
    }),
  };

  async run() {
    const { flags } = await this.parse(ShieldCommand);
    const token = flags['token']
    const amountSol = flags['amount-sol'];
    const amountSpl = flags['amount-spl'];
    const recipient = flags['recipient'];
    const minimumLamports = flags['skip-minimum-lamports'];
    const skipDecimalConversions = flags['skip-decimal-conversions'];
    
    const loader = new CustomLoader("Performing shield operation...\n");
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
      const response = await user.shield({
        token,
        recipient,
        publicAmountSpl: amountSpl ? amountSpl : 0,
        publicAmountSol: amountSol ? amountSol : 0,
        minimumLamports,
        skipDecimalConversions,
      });

      this.log(generateSolanaTransactionURL("tx", `${`${response.txHash.signatures}`}`, "custom"));

      if (!amountSol || !amountSpl ) {
        this.log(
          `\nSuccessfully shielded ${
            token.toLowerCase() === "sol" ? amountSol : amountSpl
          } ${token}`,
          "\x1b[32m✔\x1b[0m"
        );
      }
      else {
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

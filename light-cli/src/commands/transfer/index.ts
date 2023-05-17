import { Args, Command, Flags } from "@oclif/core";
import {
  generateSolanaTransactionURL,
  getUser,
  readWalletFromFile,
} from "../../utils";
import { Account } from "light-sdk";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
let circomlibjs = require("circomlibjs");

class TransferCommand extends Command {
  static description = "Transfer tokens to a recipient";

  static examples = [
    "$ light transfer --token ABC123 --amountSpl 1000000 <recipient>",
  ];

  static flags = {
    token: Flags.string({
      description: "The token to transfer",
      required: true,
    }),
    amountSpl: Flags.string({
      description: "The amount of token to transfer (SPL)",
    }),
    amountSol: Flags.string({
      description: "The amount of token to transfer (SOL)",
    }),
  };

  static args = {
    recipient: Args.string({
      name: "recipient",
      description: "The recipient address",
      required: true,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(TransferCommand);

    const { recipient } = args;
    const { token, amountSpl, amountSol } = flags;

    try {
      await readWalletFromFile();

      const user = await getUser();

      const response = await user.transfer({
        token,
        amountSpl,
        amountSol,
        recipient,
      });

      this.log(`Tokens successfully transferred to recipient: ${recipient}`);
      this.log(generateSolanaTransactionURL("tx", response.txHash, "custom"));
    } catch (error) {
      this.error(`Transfer failed: ${error}`);
    }
  }
}

TransferCommand.strict = false;

export default TransferCommand;

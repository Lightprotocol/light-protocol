import { Args, Command, Flags } from "@oclif/core";

export default class Withdraw extends Command {
  static description = "Withdraw from multisig.";

  static examples = [
    `$ oex withdraw --amount 1
Create withdrawal request for 1 SOL.`,
  ];

  static flags = {
    amount: Flags.string({
      char: "a",
      description: "Amount of SOL to withdraw",
      required: true,
    }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(Withdraw);
    this.log(`Withdrawal request for ${flags.amount} SOL.`);
  }
}

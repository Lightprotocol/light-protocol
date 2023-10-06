import { Args, Command, Flags } from "@oclif/core";

export default class Balance extends Command {
  static description = "Multisig balance.";

  static examples = [
    `$ oex balance
Multisig balance is 1 SOL.`,
  ];

  async run(): Promise<void> {
    this.log(`Multisig balance: 1 SOL.`);
  }
}

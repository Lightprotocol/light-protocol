import { Args, Command, Flags } from "@oclif/core";

export default class Approvals extends Command {
  static description = "List of approvals.";

  static examples = [
    `$ oex approvals --transaction 1
List of approvals for transaction 1.`,
  ];

  static flags = {
    transaction: Flags.string({
      char: "t",
      description: "Number of transaction",
      required: true,
    }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(Approvals);
    this.log(`List of approvals for transaction ${flags.transaction}.`);
  }
}

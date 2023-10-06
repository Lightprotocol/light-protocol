import { Args, Command, Flags } from "@oclif/core";

export default class Approve extends Command {
  static description = "Approve transaction.";

  static examples = [
    `$ oex approve --index 0
Aprrove 0.`,
  ];

  static flags = {
    index: Flags.string({
      char: "a",
      description: "Index of approval",
      required: true,
    }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(Approve);

    this.log(`Approve ${flags.index}.`);
  }
}

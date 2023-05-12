import { Args, Command, Flags } from "@oclif/core";
import { connection } from "../../utils"; // Assuming you have a file named 'utils.ts' exporting the 'connection' object

class AirdropCommand extends Command {
  static description = "Perform an airdrop to a user";

  static flags = {
    amount: Flags.integer({
      char: "a",
      description: "The amount to airdrop",
      required: true,
    }),
  };

  static examples = [`$ light-cli airdrop --amount 2000000000 <userPublicKey>`];

  static args = {
    userPublicKey: Args.string({
      name: "userPublicKey",
      description: "The public key of the user",
      required: true,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(AirdropCommand);

    const { userPublicKey } = args;
    const { amount } = flags;

    try {
      const res = await connection.requestAirdrop(userPublicKey, amount);
      await connection.confirmTransaction(res, "confirmed");

      this.log(
        `Airdrop successful for user: ${userPublicKey}, amount: ${amount}`
      );
    } catch (error) {
      this.error(`Airdrop failed: ${error.message}`);
    }
  }
}

AirdropCommand.strict = false;

export default AirdropCommand;

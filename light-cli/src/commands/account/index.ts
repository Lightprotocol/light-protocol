import { Command, Flags } from "@oclif/core";
import { getUser } from "../../utils";
import { User } from "light-sdk";
import { PublicKey } from "@solana/web3.js";

class AccountCommand extends Command {
  static description = "Get the current account details";

  async run() {
    const user: User = await getUser();

    this.log(`shielded public key: ${await user.account.getPublicKey()}`);
  }
}

export default AccountCommand;

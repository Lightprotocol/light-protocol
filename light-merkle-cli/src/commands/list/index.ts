import { initialize } from "../actions";
import { authority } from "../actions/authority";
import { pool } from "../actions/pool";
import { print } from "../actions/print";
import { verifier } from "../actions/verifier";

interface Icommands {
  name: string;
  description: string;
  option?: string[];
  action(options: any): any;
}

// TODO: Add options to all commands
// TODO: Make the help command work
export const commands: Icommands[] = [
  {
    name: "initialize",
    description: "Initializes a new Merkle tree on the Solana blockchain",
    action: initialize,
  },
  {
    name: "print",
    description: "Prints the Merkle trees that have been deployed on the Solana blockchain",
    action: print,
  },
  // {
  //   name: "authority",
  //   description: "Initializes or updates the Merkle tree authority",
  //   action: authority,
  // },
  {
    name: "verifier",
    description: "Gets or checks the verifiers for a Merkle tree",
    action: verifier,
  },
  {
    name: "pool",
    description: "Gets, sets, or changes the registered pools for a Merkle tree",
    action: pool,
  }
];

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
    name: "print",
    description: "Prints the Merkle trees that have been deployed on the Solana blockchain",
    action: print,
  }
];

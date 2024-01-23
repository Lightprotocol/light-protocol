import { Idl } from "@coral-xyz/anchor";

export type ProgramParameters = {
  verifierIdl: Idl;
  inputs: any; // object of proof and other inputs
  path: string;
  accounts?: any; // object of accounts
  circuitName: string;
};

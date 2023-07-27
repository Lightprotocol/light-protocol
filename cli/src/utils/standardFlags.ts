import { Flags } from "@oclif/core";

export const standardFlags = {
  skipFetchBalance: Flags.boolean({
    char: "b",
    description: "Skip fetching the most recent balance prior to the operation",
    required: false,
    default: false,
    parse: async () => true,
  }),
};

export const confirmOptionsFlags = {
  spendable: Flags.boolean({
    char: "s",
    description: "Fetch the most recent balance prior to the operation",
    required: false,
    default: false,
    parse: async () => true,
  }),
  finalized: Flags.boolean({
    char: "f",
    description: "Fetch the most recent balance prior to the operation",
    required: false,
    default: false,
    parse: async () => true,
  }),
};

import { Idl } from "@coral-xyz/anchor";
export type ProgramParameters = {
    verifierIdl: Idl;
    inputs: any;
    path: string;
    accounts?: any;
    circuitName: string;
};

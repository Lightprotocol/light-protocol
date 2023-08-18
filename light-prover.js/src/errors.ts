export class MetaError extends Error {
    code: string;
    codeMessage?: string;
    functionName: string;

    constructor(code: string, functionName: string, codeMessage?: string) {
        super(`${code}: ${codeMessage}`);

        this.codeMessage = codeMessage;
        this.code = code;
        this.functionName = functionName;
    }
}

export class VerifierError extends MetaError {}

export enum VerifierErrorCode {
    PUBLIC_INPUTS_UNDEFINED = "PUBLIC_INPUTS_UNDEFINED",
    INVALID_INPUTS_NUMBER = "INVALID_INPUTS_NUMBER",
    ENCRYPTING_UTXOS_UNDEFINED = "ENCRYPTING_UTXOS_UNDEFINED",
}
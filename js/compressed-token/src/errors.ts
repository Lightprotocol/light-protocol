export enum TokenUtxoErrorCode {}

export enum UtilsErrorCode {}

class MetaError extends Error {
  code: string;
  functionName: string;
  codeMessage?: string;

  constructor(code: string, functionName: string, codeMessage?: string) {
    super(`${code}: ${codeMessage}`);
    this.code = code;
    this.functionName = functionName;
    this.codeMessage = codeMessage;
  }
}

export class TokenUtxoError extends MetaError {}

export class UtilsError extends MetaError {}

export enum UtxoErrorCode {
  APP_DATA_FROM_BYTES_FUNCTION_UNDEFINED = "APP_DATA_FROM_BYTES_FUNCTION_UNDEFINED",
  INVALID_ASSET_OR_AMOUNTS_LENGTH = "INVALID_ASSET_OR_AMOUNTS_LENGTH",
  EXCEEDED_MAX_ASSETS = "EXCEEDED_MAX_ASSETS",
  NEGATIVE_AMOUNT = "NEGATIVE_AMOUNT",
  NOT_U64 = "NOT_U64",
  BLINDING_EXCEEDS_SIZE = "BLINDING_EXCEEDS_SIZE",
}

/** Thrown when something fails in the Utxo class.
 *
 * @note
 **/
export class UtxoError extends Error {
  name = this.constructor.name;
  code: string;
  codeMessage: string;
  codeStack: string | null;

  constructor(code: string, codeMessage: string, codeStack?: string) {
    super(`Utxo error ${code}: ${codeMessage}`);
    this.code = code;
    this.codeMessage = codeMessage;
    this.codeStack = codeStack || null;
  }
}

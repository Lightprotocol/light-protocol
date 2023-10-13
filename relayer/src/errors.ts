import { MetaError } from "@lightprotocol/zk.js";

export enum EnvironmentVariableErrorCode {
  INVALID_VARIABLE = "INVALID_VARIABLE",
  VARIABLE_NOT_SET = "VARIABLE_NOT_SET",
  RELAYER_SIGNER_UNDEFINED = "RELAYER_SIGNER_UNDEFINED",
  RELAYER_RECIPIENT_UNDEFINED = "RELAYER_RECIPIENT_UNDEFINED",
}
export enum RelayErrorCode {
  NO_INSTRUCTIONS_PROVIDED = "NO_INSTRUCTIONS_PROVIDED",
}
export enum AccountErrorCode {
  LOOK_UP_TABLE_NOT_INITIALIZED = "LOOK_UP_TABLE_NOT_INITIALIZED",
  RELAYER_NOT_FUNDED = "RELAYER_NOT_FUNDED",
  RELAYER_RECIPIENT_NOT_FUNDED = "RELAYER_RECIPIENT_NOT_FUNDED",
}

export enum RedisErrorCode {
  NO_REDIS_CONNECTION = "NO_REDIS_CONNECTION",
}

export class RedisError extends MetaError {}
export class EnvironmentVariableError extends MetaError {}
export class AccountError extends MetaError {}
export class RelayError extends MetaError {}

import { MetaError } from "@lightprotocol/zk.js";

export enum EnvironmentVariableErrorCode {
  INVALID_VARIABLE = "INVALID_VARIABLE",
  VARIABLE_NOT_SET = "VARIABLE_NOT_SET",
  RPC_SIGNER_UNDEFINED = "RPC_SIGNER_UNDEFINED",
  RPC_RECIPIENT_UNDEFINED = "RPC_RECIPIENT_UNDEFINED",
}
export enum RpcrorCode {
  NO_INSTRUCTIONS_PROVIDED = "NO_INSTRUCTIONS_PROVIDED",
}
export enum AccountErrorCode {
  LOOK_UP_TABLE_NOT_INITIALIZED = "LOOK_UP_TABLE_NOT_INITIALIZED",
  RPC_NOT_FUNDED = "RPC_NOT_FUNDED",
  RPC_RECIPIENT_NOT_FUNDED = "RPC_RECIPIENT_NOT_FUNDED",
}

export enum RedisErrorCode {
  NO_REDIS_CONNECTION = "NO_REDIS_CONNECTION",
}

export class RedisError extends MetaError {}
export class EnvironmentVariableError extends MetaError {}
export class AccountError extends MetaError {}
export class Rpcror extends MetaError {}

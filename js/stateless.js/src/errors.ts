export enum UtxoErrorCode {
  NEGATIVE_LAMPORTS = 'NEGATIVE_LAMPORTS',
  NOT_U64 = 'NOT_U64',
  BLINDING_EXCEEDS_FIELD_SIZE = 'BLINDING_EXCEEDS_FIELD_SIZE',
}

export enum SelectInUtxosErrorCode {
  FAILED_TO_FIND_UTXO_COMBINATION = 'FAILED_TO_FIND_UTXO_COMBINATION',
  INVALID_NUMBER_OF_IN_UTXOS = 'INVALID_NUMBER_OF_IN_UTXOS',
}

export enum CreateUtxoErrorCode {
  OWNER_UNDEFINED = 'OWNER_UNDEFINED',
  INVALID_OUTPUT_UTXO_LENGTH = 'INVALID_OUTPUT_UTXO_LENGTH',
  UTXO_DATA_UNDEFINED = 'UTXO_DATA_UNDEFINED',
}

export enum RpcErrorCode {
  CONNECTION_UNDEFINED = 'CONNECTION_UNDEFINED',
  RPC_PUBKEY_UNDEFINED = 'RPC_PUBKEY_UNDEFINED',
  RPC_METHOD_NOT_IMPLEMENTED = 'RPC_METHOD_NOT_IMPLEMENTED',
  RPC_INVALID = 'RPC_INVALID',
}

export enum LookupTableErrorCode {
  LOOK_UP_TABLE_UNDEFINED = 'LOOK_UP_TABLE_UNDEFINED',
  LOOK_UP_TABLE_NOT_INITIALIZED = 'LOOK_UP_TABLE_NOT_INITIALIZED',
}

export enum HashErrorCode {
  NO_POSEIDON_HASHER_PROVIDED = 'NO_POSEIDON_HASHER_PROVIDED',
}

export enum ProofErrorCode {
  INVALID_PROOF = 'INVALID_PROOF',
  PROOF_INPUT_UNDEFINED = 'PROOF_INPUT_UNDEFINED',
  PROOF_GENERATION_FAILED = 'PROOF_GENERATION_FAILED',
}

export enum MerkleTreeErrorCode {
  MERKLE_TREE_NOT_INITIALIZED = 'MERKLE_TREE_NOT_INITIALIZED',
  SOL_MERKLE_TREE_UNDEFINED = 'SOL_MERKLE_TREE_UNDEFINED',
  MERKLE_TREE_UNDEFINED = 'MERKLE_TREE_UNDEFINED',
  INPUT_UTXO_NOT_INSERTED_IN_MERKLE_TREE = 'INPUT_UTXO_NOT_INSERTED_IN_MERKLE_TREE',
  MERKLE_TREE_INDEX_UNDEFINED = 'MERKLE_TREE_INDEX_UNDEFINED',
  MERKLE_TREE_SET_SPACE_UNDEFINED = 'MERKLE_TREE_SET_SPACE_UNDEFINED',
}

export enum UtilsErrorCode {
  ACCOUNT_NAME_UNDEFINED_IN_IDL = 'ACCOUNT_NAME_UNDEFINED_IN_IDL',
  PROPERTY_UNDEFINED = 'PROPERTY_UNDEFINED',
  LOOK_UP_TABLE_CREATION_FAILED = 'LOOK_UP_TABLE_CREATION_FAILED',
  UNSUPPORTED_ARCHITECTURE = 'UNSUPPORTED_ARCHITECTURE',
  UNSUPPORTED_PLATFORM = 'UNSUPPORTED_PLATFORM',
  ACCOUNTS_UNDEFINED = 'ACCOUNTS_UNDEFINED',
  INVALID_NUMBER = 'INVALID_NUMBER',
}

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

export class UtxoError extends MetaError {}

export class SelectInUtxosError extends MetaError {}

export class CreateUtxoError extends MetaError {}

export class RpcError extends MetaError {}

export class LookupTableError extends MetaError {}

export class HashError extends MetaError {}

export class ProofError extends MetaError {}

export class MerkleTreeError extends MetaError {}

export class UtilsError extends MetaError {}

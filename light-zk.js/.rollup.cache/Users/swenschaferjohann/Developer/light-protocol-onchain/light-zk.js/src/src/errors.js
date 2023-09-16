export var UtxoErrorCode;
(function (UtxoErrorCode) {
    UtxoErrorCode["APP_DATA_IDL_UNDEFINED"] = "APP_DATA_IDL_UNDEFINED";
    UtxoErrorCode["INVALID_ASSET_OR_AMOUNTS_LENGTH"] = "INVALID_ASSET_OR_AMOUNTS_LENGTH";
    UtxoErrorCode["EXCEEDED_MAX_ASSETS"] = "EXCEEDED_MAX_ASSETS";
    UtxoErrorCode["NEGATIVE_AMOUNT"] = "NEGATIVE_AMOUNT";
    UtxoErrorCode["NON_ZERO_AMOUNT"] = "NON_ZERO_AMOUNT";
    UtxoErrorCode["POSITIVE_AMOUNT"] = "POSITIVE_AMOUNT";
    UtxoErrorCode["NOT_U64"] = "NOT_U64";
    UtxoErrorCode["BLINDING_EXCEEDS_FIELD_SIZE"] = "BLINDING_EXCEEDS_FIELD_SIZE";
    UtxoErrorCode["INDEX_NOT_PROVIDED"] = "INDEX_NOT_PROVIDED";
    UtxoErrorCode["ACCOUNT_HAS_NO_PRIVKEY"] = "ACCOUNT_HAS_NO_PRIVKEY";
    UtxoErrorCode["ASSET_NOT_FOUND"] = "ASSET_NOT_FOUND";
    UtxoErrorCode["APP_DATA_UNDEFINED"] = "APP_DATA_UNDEFINED";
    UtxoErrorCode["APP_DATA_IDL_DOES_NOT_HAVE_ACCOUNTS"] = "APP_DATA_IDL_DOES_NOT_HAVE_ACCOUNTS";
    UtxoErrorCode["UTXO_APP_DATA_NOT_FOUND_IN_IDL"] = "UTXO_APP_DATA_NOT_FOUND_IN_IDL";
    UtxoErrorCode["AES_SECRET_UNDEFINED"] = "AES_SECRET_UNDEFINED";
    UtxoErrorCode["INVALID_NONCE_LENGHT"] = "INVALID_NONCE_LENGHT";
    UtxoErrorCode["MERKLE_TREE_PDA_PUBLICKEY_UNDEFINED"] = "MERKLE_TREE_PDA_PUBLICKEY_UNDEFINED";
    UtxoErrorCode["TRANSACTION_INDEX_UNDEFINED"] = "TRANSACTION_INDEX_UNDEFINED";
    UtxoErrorCode["INVALID_APP_DATA"] = "INVALID_APP_DATA";
    UtxoErrorCode["VERIFIER_INDEX_NOT_FOUND"] = "VERIFIER_INDEX_NOT_FOUND";
    UtxoErrorCode["ASSET_UNDEFINED"] = "ASSET_UNDEFINED";
    UtxoErrorCode["INVALID_APP_DATA_IDL"] = "INVALID_APP_DATA_IDL";
    UtxoErrorCode["INVALID_IV"] = "INVALID_IV";
})(UtxoErrorCode || (UtxoErrorCode = {}));
export var UserErrorCode;
(function (UserErrorCode) {
    UserErrorCode["NO_WALLET_PROVIDED"] = "NO_WALLET_PROVIDED";
    UserErrorCode["LOAD_ERROR"] = "LOAD_ERROR";
    UserErrorCode["PROVIDER_NOT_INITIALIZED"] = "PROVIDER_NOT_INITIALIZED";
    UserErrorCode["UTXOS_NOT_INITIALIZED"] = "UTXOS_NOT_INITIALIZED";
    UserErrorCode["USER_ACCOUNT_NOT_INITIALIZED"] = "USER_ACCOUNT_NOT_INITIALIZED";
    UserErrorCode["TOKEN_NOT_FOUND"] = "TOKEN_NOT_FOUND";
    UserErrorCode["NO_AMOUNTS_PROVIDED"] = "NO_AMOUNTS_PROVIDED";
    UserErrorCode["APPROVE_ERROR"] = "APPROVE_ERROR";
    UserErrorCode["INSUFFICIENT_BAlANCE"] = "INSUFFICIENT_BAlANCE";
    UserErrorCode["ASSOCIATED_TOKEN_ACCOUNT_DOESNT_EXIST"] = "ASSOCIATED_TOKEN_ACCOUNT_DOESNT_EXIST";
    UserErrorCode["TOKEN_ACCOUNT_DEFINED"] = "TOKEN_ACCOUNT_DEFINED";
    UserErrorCode["SHIELDED_RECIPIENT_UNDEFINED"] = "SHIELDED_RECIPIENT_UNDEFINED";
    UserErrorCode["TOKEN_UNDEFINED"] = "TOKEN_UNDEFINED";
    UserErrorCode["INVALID_TOKEN"] = "INVALID_TOKEN";
    UserErrorCode["TRANSACTION_PARAMTERS_UNDEFINED"] = "TRANSACTION_PARAMTERS_UNDEFINED";
    UserErrorCode["SPL_FUNDS_NOT_APPROVED"] = "SPL_FUNDS_NOT_APPROVED";
    UserErrorCode["TRANSACTION_UNDEFINED"] = "TRANSACTION_UNDEFINED";
    UserErrorCode["VERIFIER_IS_NOT_APP_ENABLED"] = "VERIFIER_IS_NOT_APP_ENABLED";
    UserErrorCode["EMPTY_INBOX"] = "EMPTY_INBOX";
    UserErrorCode["COMMITMENT_NOT_FOUND"] = "COMMITMENT_NOT_FOUND";
    UserErrorCode["NO_COMMITMENTS_PROVIDED"] = "NO_COMMITMENTS_PROVIDED";
    UserErrorCode["TOO_MANY_COMMITMENTS"] = "TOO_MANY_COMMITMENTS";
    UserErrorCode["MAX_STORAGE_MESSAGE_SIZE_EXCEEDED"] = "MAX_STORAGE_MESSAGE_SIZE_EXCEEDED";
    UserErrorCode["APP_UTXO_UNDEFINED"] = "APP_UTXO_UNDEFINED";
    UserErrorCode["ENCRYPTION_FAILED"] = "ENCRYPTION_FAILED";
    UserErrorCode["ADD_IN_UTXOS_FALSE"] = "ADD_IN_UTXOS_FALSE";
})(UserErrorCode || (UserErrorCode = {}));
export var SelectInUtxosErrorCode;
(function (SelectInUtxosErrorCode) {
    SelectInUtxosErrorCode["INVALID_NUMER_OF_MINTS"] = "INVALID_NUMER_OF_MINTS";
    SelectInUtxosErrorCode["FAILED_TO_SELECT_SOL_UTXO"] = "FAILED_TO_SELECT_SOL_UTXO";
    SelectInUtxosErrorCode["FAILED_TO_FIND_UTXO_COMBINATION"] = "FAILED_TO_FIND_UTXO_COMBINATION";
    SelectInUtxosErrorCode["INVALID_NUMBER_OF_IN_UTXOS"] = "INVALID_NUMBER_OF_IN_UTXOS";
})(SelectInUtxosErrorCode || (SelectInUtxosErrorCode = {}));
export var TokenUtxoBalanceErrorCode;
(function (TokenUtxoBalanceErrorCode) {
    TokenUtxoBalanceErrorCode["UTXO_UNDEFINED"] = "UTXO_UNDEFINED";
})(TokenUtxoBalanceErrorCode || (TokenUtxoBalanceErrorCode = {}));
export var RelayerErrorCode;
(function (RelayerErrorCode) {
    RelayerErrorCode["RELAYER_FEE_UNDEFINED"] = "RELAYER_FEE_UNDEFINED";
    RelayerErrorCode["RELAYER_PUBKEY_UNDEFINED"] = "RELAYER_PUBKEY_UNDEFINED";
    RelayerErrorCode["LOOK_UP_TABLE_UNDEFINED"] = "LOOK_UP_TABLE_UNDEFINED";
    RelayerErrorCode["RELAYER_RECIPIENT_UNDEFINED"] = "RELAYER_RECIPIENT_UNDEFINED";
})(RelayerErrorCode || (RelayerErrorCode = {}));
export var CreateUtxoErrorCode;
(function (CreateUtxoErrorCode) {
    CreateUtxoErrorCode["INVALID_NUMER_OF_RECIPIENTS"] = "INVALID_NUMER_OF_RECIPIENTS";
    CreateUtxoErrorCode["INVALID_RECIPIENT_MINT"] = "INVALID_RECIPIENT_MINT";
    CreateUtxoErrorCode["RECIPIENTS_SUM_AMOUNT_MISSMATCH"] = "RECIPIENTS_SUM_AMOUNT_MISSMATCH";
    CreateUtxoErrorCode["NO_PUBLIC_AMOUNTS_PROVIDED"] = "NO_PUBLIC_AMOUNTS_PROVIDED";
    CreateUtxoErrorCode["NO_PUBLIC_MINT_PROVIDED"] = "NO_PUBLIC_MINT_PROVIDED";
    CreateUtxoErrorCode["MINT_UNDEFINED"] = "MINT_UNDEFINED";
    CreateUtxoErrorCode["SPL_AMOUNT_UNDEFINED"] = "SPL_AMOUNT_UNDEFINED";
    CreateUtxoErrorCode["ACCOUNT_UNDEFINED"] = "ACCOUNT_UNDEFINED";
    CreateUtxoErrorCode["INVALID_OUTPUT_UTXO_LENGTH"] = "INVALID_OUTPUT_UTXO_LENGTH";
    CreateUtxoErrorCode["RELAYER_FEE_DEFINED"] = "RELAYER_FEE_DEFINED";
    CreateUtxoErrorCode["PUBLIC_SOL_AMOUNT_UNDEFINED"] = "PUBLIC_SOL_AMOUNT_UNDEFINED";
    CreateUtxoErrorCode["PUBLIC_SPL_AMOUNT_UNDEFINED"] = "PUBLIC_SPL_AMOUNT_UNDEFINED";
})(CreateUtxoErrorCode || (CreateUtxoErrorCode = {}));
export var AccountErrorCode;
(function (AccountErrorCode) {
    AccountErrorCode["INVALID_SEED_SIZE"] = "INVALID_SEED_SIZE";
    AccountErrorCode["SEED_UNDEFINED"] = "SEED_UNDEFINED";
    AccountErrorCode["SEED_DEFINED"] = "SEED_DEFINED";
    AccountErrorCode["ENCRYPTION_PRIVATE_KEY_UNDEFINED"] = "ENCRYPTION_PRIVATE_KEY_UNDEFINED";
    AccountErrorCode["PRIVATE_KEY_UNDEFINED"] = "PRIVATE_KEY_UNDEFINED";
    AccountErrorCode["POSEIDON_EDDSA_KEYPAIR_UNDEFINED"] = "POSEIDON_EDDSA_KEYPAIR_UNDEFINED";
    AccountErrorCode["POSEIDON_EDDSA_GET_PUBKEY_FAILED"] = "POSEIDON_EDDSA_GET_PUBKEY_FAILED";
    AccountErrorCode["PUBLIC_KEY_UNDEFINED"] = "PUBLIC_KEY_UNDEFINED";
    AccountErrorCode["AES_SECRET_UNDEFINED"] = "AES_SECRET_UNDEFINED";
    AccountErrorCode["INVALID_PUBLIC_KEY_SIZE"] = "INVALID_PUBLIC_KEY_SIZE";
})(AccountErrorCode || (AccountErrorCode = {}));
export var ProviderErrorCode;
(function (ProviderErrorCode) {
    ProviderErrorCode["SOL_MERKLE_TREE_UNDEFINED"] = "SOL_MERKLE_TREE_UNDEFINED";
    ProviderErrorCode["ANCHOR_PROVIDER_UNDEFINED"] = "ANCHOR_PROVIDER_UNDEFINED";
    ProviderErrorCode["PROVIDER_UNDEFINED"] = "PROVIDER_UNDEFINED";
    ProviderErrorCode["WALLET_UNDEFINED"] = "WALLET_UNDEFINED";
    ProviderErrorCode["NODE_WALLET_UNDEFINED"] = "NODE_WALLET_UNDEFINED";
    ProviderErrorCode["URL_UNDEFINED"] = "URL_UNDEFINED";
    ProviderErrorCode["CONNECTION_UNDEFINED"] = "CONNECTION_UNDEFINED";
    ProviderErrorCode["CONNECTION_DEFINED"] = "CONNECTION_DEFINED";
    ProviderErrorCode["KEYPAIR_UNDEFINED"] = "KEYPAIR_UNDEFINED";
    ProviderErrorCode["WALLET_DEFINED"] = "WALLET_DEFINED";
    ProviderErrorCode["MERKLE_TREE_NOT_INITIALIZED"] = "MERKLE_TREE_NOT_INITIALIZED";
    ProviderErrorCode["LOOK_UP_TABLE_NOT_INITIALIZED"] = "LOOK_UP_TABLE_NOT_INITIALIZED";
})(ProviderErrorCode || (ProviderErrorCode = {}));
export var SolMerkleTreeErrorCode;
(function (SolMerkleTreeErrorCode) {
    SolMerkleTreeErrorCode["MERKLE_TREE_UNDEFINED"] = "MERKLE_TREE_UNDEFINED";
})(SolMerkleTreeErrorCode || (SolMerkleTreeErrorCode = {}));
export var TransactionParametersErrorCode;
(function (TransactionParametersErrorCode) {
    TransactionParametersErrorCode["NO_VERIFIER_IDL_PROVIDED"] = "NO_VERIFIER_IDL_PROVIDED";
    TransactionParametersErrorCode["NO_POSEIDON_HASHER_PROVIDED"] = "NO_POSEIDON_HASHER_PROVIDED";
    TransactionParametersErrorCode["NO_ACTION_PROVIDED"] = "NO_ACTION_PROVIDED";
    TransactionParametersErrorCode["PUBLIC_AMOUNT_NEGATIVE"] = "PUBLIC_AMOUNT_NEGATIVE";
    TransactionParametersErrorCode["SOL_RECIPIENT_DEFINED"] = "SOL_RECIPIENT_DEFINED";
    TransactionParametersErrorCode["SPL_RECIPIENT_DEFINED"] = "SPL_RECIPIENT_DEFINED";
    TransactionParametersErrorCode["PUBLIC_AMOUNT_NOT_U64"] = "PUBLIC_AMOUNT_NOT_U64";
    TransactionParametersErrorCode["RELAYER_DEFINED"] = "RELAYER_DEFINED";
    TransactionParametersErrorCode["INVALID_PUBLIC_AMOUNT"] = "INVALID_PUBLIC_AMOUNT";
    TransactionParametersErrorCode["SOL_SENDER_DEFINED"] = "SOL_SENDER_DEFINED";
    TransactionParametersErrorCode["SPL_SENDER_DEFINED"] = "SPL_SENDER_DEFINED";
    TransactionParametersErrorCode["PUBLIC_AMOUNT_SPL_NOT_ZERO"] = "PUBLIC_AMOUNT_SPL_NOT_ZERO";
    TransactionParametersErrorCode["PUBLIC_AMOUNT_SOL_NOT_ZERO"] = "PUBLIC_AMOUNT_SOL_NOT_ZERO";
    TransactionParametersErrorCode["LOOK_UP_TABLE_UNDEFINED"] = "LOOK_UP_TABLE_UNDEFINED";
    TransactionParametersErrorCode["INVALID_NUMBER_OF_NONCES"] = "INVALID_NUMBER_OF_NONCES";
    TransactionParametersErrorCode["VERIFIER_IDL_UNDEFINED"] = "VERIFIER_IDL_UNDEFINED";
    TransactionParametersErrorCode["RELAYER_INVALID"] = "RELAYER_INVALID";
    TransactionParametersErrorCode["UTXO_IDLS_UNDEFINED"] = "UTXO_IDLS_UNDEFINED";
    TransactionParametersErrorCode["EVENT_MERKLE_TREE_UNDEFINED"] = "EVENT_MERKLE_TREE_UNDEFINED";
    TransactionParametersErrorCode["MESSAGE_UNDEFINED"] = "MESSAGE_UNDEFINED";
    TransactionParametersErrorCode["PROGRAM_ID_CONSTANT_UNDEFINED"] = "PROGRAM_ID_CONSTANT_UNDEFINED";
    TransactionParametersErrorCode["ENCRYPTED_UTXOS_TOO_LONG"] = "ENCRYPTED_UTXOS_TOO_LONG";
})(TransactionParametersErrorCode || (TransactionParametersErrorCode = {}));
export var TransactionErrorCode;
(function (TransactionErrorCode) {
    TransactionErrorCode["PROVIDER_UNDEFINED"] = "PROVIDER_UNDEFINED";
    TransactionErrorCode["ROOT_INDEX_NOT_FETCHED"] = "ROOT_INDEX_NOT_FETCHED";
    TransactionErrorCode["REMAINING_ACCOUNTS_NOT_CREATED"] = "REMAINING_ACCOUNTS_NOT_CREATED";
    TransactionErrorCode["TRANSACTION_INPUTS_UNDEFINED"] = "TRANSACTION_INPUTS_UNDEFINED";
    TransactionErrorCode["WALLET_RELAYER_INCONSISTENT"] = "WALLET_RELAYER_INCONSISTENT";
    TransactionErrorCode["TX_PARAMETERS_UNDEFINED"] = "TX_PARAMETERS_UNDEFINED";
    TransactionErrorCode["APP_PARAMETERS_UNDEFINED"] = "APP_PARAMETERS_UNDEFINED";
    TransactionErrorCode["RELAYER_UNDEFINED"] = "TransactionParameters.relayer is undefined";
    TransactionErrorCode["WALLET_UNDEFINED"] = "WALLET_UNDEFINED";
    TransactionErrorCode["NO_UTXOS_PROVIDED"] = "NO_UTXOS_PROVIDED";
    TransactionErrorCode["EXCEEDED_MAX_ASSETS"] = "EXCEEDED_MAX_ASSETS";
    TransactionErrorCode["VERIFIER_PROGRAM_UNDEFINED"] = "VERIFIER_PROGRAM_UNDEFINED";
    TransactionErrorCode["SPL_RECIPIENT_UNDEFINED"] = "SPL_RECIPIENT_UNDEFINED";
    TransactionErrorCode["SOL_RECIPIENT_UNDEFINED"] = "SOL_RECIPIENT_UNDEFINED";
    TransactionErrorCode["SPL_SENDER_UNDEFINED"] = "SPL_SENDER_UNDEFINED";
    TransactionErrorCode["SOL_SENDER_UNDEFINED"] = "SOL_SENDER_UNDEFINED";
    TransactionErrorCode["ASSET_PUBKEYS_UNDEFINED"] = "ASSET_PUBKEYS_UNDEFINED";
    TransactionErrorCode["ACTION_IS_NO_WITHDRAWAL"] = "ACTION_IS_NO_WITHDRAWAL";
    TransactionErrorCode["ACTION_IS_NO_DEPOSIT"] = "ACTION_IS_NO_DEPOSIT";
    TransactionErrorCode["INPUT_UTXOS_UNDEFINED"] = "INPUT_UTXOS_UNDEFINED";
    TransactionErrorCode["OUTPUT_UTXOS_UNDEFINED"] = "OUTPUT_UTXOS_UNDEFINED";
    TransactionErrorCode["GET_MINT_FAILED"] = "GET_MINT_FAILED";
    TransactionErrorCode["VERIFIER_IDL_UNDEFINED"] = "VERIFIER_UNDEFINED";
    TransactionErrorCode["PROOF_INPUT_UNDEFINED"] = "PROOF_INPUT_UNDEFINED";
    TransactionErrorCode["NO_PARAMETERS_PROVIDED"] = "NO_PARAMETERS_PROVIDED";
    TransactionErrorCode["ROOT_NOT_FOUND"] = "ROOT_NOT_FOUND";
    TransactionErrorCode["VERIFIER_CONFIG_UNDEFINED"] = "VERIFIER_CONFIG_UNDEFINED";
    TransactionErrorCode["RELAYER_FEE_UNDEFINED"] = "RELAYER_FEE_UNDEFINED";
    TransactionErrorCode["ENCRYPTING_UTXOS_FAILED"] = "ENCRYPTING_UTXOS_FAILED";
    TransactionErrorCode["GET_INSTRUCTIONS_FAILED"] = "GET_INSTRUCTIONS_FAILED";
    TransactionErrorCode["SEND_TRANSACTION_FAILED"] = "SEND_TRANSACTION_FAILED";
    TransactionErrorCode["PUBLIC_INPUTS_UNDEFINED"] = "PUBLIC_INPUTS_UNDEFINED";
    TransactionErrorCode["MERKLE_TREE_PROGRAM_UNDEFINED"] = "MERKLE_TREE_PROGRAM_UNDEFINED";
    TransactionErrorCode["INPUT_UTXO_NOT_INSERTED_IN_MERKLE_TREE"] = "INPUT_UTXO_NOT_INSERTED_IN_MERKLE_TREE";
    TransactionErrorCode["INVALID_PROOF"] = "INVALID_PROOF";
    TransactionErrorCode["POSEIDON_HASHER_UNDEFINED"] = "POSEIDON_HASHER_UNDEFINED";
    TransactionErrorCode["PROOF_GENERATION_FAILED"] = "PROOF_GENERATION_FAILED";
    TransactionErrorCode["INVALID_VERIFIER_SELECTED"] = "INVALID_VERIFIER_SELECTED";
    TransactionErrorCode["MESSAGE_UNDEFINED"] = "MESSAGE_UNDEFINED";
    TransactionErrorCode["UNIMPLEMENTED"] = "UNIMPLEMENTED";
    TransactionErrorCode["TX_INTEGRITY_HASH_UNDEFINED"] = "TX_INTEGRITY_HASH_UNDEFINED";
    TransactionErrorCode["GET_USER_TRANSACTION_HISTORY_FAILED"] = "GET_USER_TRANSACTION_HISTORY_FAILED";
    TransactionErrorCode["FIRST_PATH_APP_UNDEFINED"] = "FIRST_PATH_APP_UNDEFINED";
})(TransactionErrorCode || (TransactionErrorCode = {}));
export var UtilsErrorCode;
(function (UtilsErrorCode) {
    UtilsErrorCode["ACCOUNT_NAME_UNDEFINED_IN_IDL"] = "ACCOUNT_NAME_UNDEFINED_IN_IDL";
    UtilsErrorCode["PROPERTY_UNDEFINED"] = "PROPERTY_UNDEFINED";
})(UtilsErrorCode || (UtilsErrorCode = {}));
export var ProgramUtxoBalanceErrorCode;
(function (ProgramUtxoBalanceErrorCode) {
    ProgramUtxoBalanceErrorCode["INVALID_PROGRAM_ADDRESS"] = "INVALID_PROGRAM_ADDRESS";
    ProgramUtxoBalanceErrorCode["TOKEN_DATA_NOT_FOUND"] = "TOKEN_DATA_NOT_FOUND";
})(ProgramUtxoBalanceErrorCode || (ProgramUtxoBalanceErrorCode = {}));
export class MetaError extends Error {
    constructor(code, functionName, codeMessage) {
        super(`${code}: ${codeMessage}`);
        this.codeMessage = codeMessage;
        this.code = code;
        this.functionName = functionName;
    }
}
/**
 * @description Thrown when something fails in the Transaction class.
 **/
export class TransactionError extends MetaError {
}
export class TransactionParametersError extends MetaError {
}
/**
 * @description Thrown when something fails in the Utxo class.
 **/
export class UtxoError extends MetaError {
}
export class AccountError extends MetaError {
}
export class RelayerError extends MetaError {
}
export class CreateUtxoError extends MetaError {
}
export class ProviderError extends MetaError {
}
export class SelectInUtxosError extends MetaError {
}
export class UserError extends MetaError {
}
export class UtilsError extends MetaError {
}
export class TokenUtxoBalanceError extends MetaError {
}
export class ProgramUtxoBalanceError extends MetaError {
}
//# sourceMappingURL=errors.js.map
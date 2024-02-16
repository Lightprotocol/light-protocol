"use strict";
var __extends = (this && this.__extends) || (function () {
    var extendStatics = function (d, b) {
        extendStatics = Object.setPrototypeOf ||
            ({ __proto__: [] } instanceof Array && function (d, b) { d.__proto__ = b; }) ||
            function (d, b) { for (var p in b) if (Object.prototype.hasOwnProperty.call(b, p)) d[p] = b[p]; };
        return extendStatics(d, b);
    };
    return function (d, b) {
        if (typeof b !== "function" && b !== null)
            throw new TypeError("Class extends value " + String(b) + " is not a constructor or null");
        extendStatics(d, b);
        function __() { this.constructor = d; }
        d.prototype = b === null ? Object.create(b) : (__.prototype = b.prototype, new __());
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.UtilsError = exports.MerkleTreeError = exports.ProofError = exports.HashError = exports.LookupTableError = exports.RpcError = exports.CreateUtxoError = exports.SelectInUtxosError = exports.UtxoError = exports.UtilsErrorCode = exports.MerkleTreeErrorCode = exports.ProofErrorCode = exports.HashErrorCode = exports.LookupTableErrorCode = exports.RpcErrorCode = exports.CreateUtxoErrorCode = exports.SelectInUtxosErrorCode = exports.UtxoErrorCode = void 0;
var UtxoErrorCode;
(function (UtxoErrorCode) {
    UtxoErrorCode["NEGATIVE_LAMPORTS"] = "NEGATIVE_LAMPORTS";
    UtxoErrorCode["NOT_U64"] = "NOT_U64";
    UtxoErrorCode["BLINDING_EXCEEDS_FIELD_SIZE"] = "BLINDING_EXCEEDS_FIELD_SIZE";
})(UtxoErrorCode || (exports.UtxoErrorCode = UtxoErrorCode = {}));
var SelectInUtxosErrorCode;
(function (SelectInUtxosErrorCode) {
    SelectInUtxosErrorCode["FAILED_TO_FIND_UTXO_COMBINATION"] = "FAILED_TO_FIND_UTXO_COMBINATION";
    SelectInUtxosErrorCode["INVALID_NUMBER_OF_IN_UTXOS"] = "INVALID_NUMBER_OF_IN_UTXOS";
})(SelectInUtxosErrorCode || (exports.SelectInUtxosErrorCode = SelectInUtxosErrorCode = {}));
var CreateUtxoErrorCode;
(function (CreateUtxoErrorCode) {
    CreateUtxoErrorCode["OWNER_UNDEFINED"] = "OWNER_UNDEFINED";
    CreateUtxoErrorCode["INVALID_OUTPUT_UTXO_LENGTH"] = "INVALID_OUTPUT_UTXO_LENGTH";
    CreateUtxoErrorCode["UTXO_DATA_UNDEFINED"] = "UTXO_DATA_UNDEFINED";
})(CreateUtxoErrorCode || (exports.CreateUtxoErrorCode = CreateUtxoErrorCode = {}));
var RpcErrorCode;
(function (RpcErrorCode) {
    RpcErrorCode["CONNECTION_UNDEFINED"] = "CONNECTION_UNDEFINED";
    RpcErrorCode["RPC_PUBKEY_UNDEFINED"] = "RPC_PUBKEY_UNDEFINED";
    RpcErrorCode["RPC_METHOD_NOT_IMPLEMENTED"] = "RPC_METHOD_NOT_IMPLEMENTED";
    RpcErrorCode["RPC_INVALID"] = "RPC_INVALID";
})(RpcErrorCode || (exports.RpcErrorCode = RpcErrorCode = {}));
var LookupTableErrorCode;
(function (LookupTableErrorCode) {
    LookupTableErrorCode["LOOK_UP_TABLE_UNDEFINED"] = "LOOK_UP_TABLE_UNDEFINED";
    LookupTableErrorCode["LOOK_UP_TABLE_NOT_INITIALIZED"] = "LOOK_UP_TABLE_NOT_INITIALIZED";
})(LookupTableErrorCode || (exports.LookupTableErrorCode = LookupTableErrorCode = {}));
var HashErrorCode;
(function (HashErrorCode) {
    HashErrorCode["NO_POSEIDON_HASHER_PROVIDED"] = "NO_POSEIDON_HASHER_PROVIDED";
})(HashErrorCode || (exports.HashErrorCode = HashErrorCode = {}));
var ProofErrorCode;
(function (ProofErrorCode) {
    ProofErrorCode["INVALID_PROOF"] = "INVALID_PROOF";
    ProofErrorCode["PROOF_INPUT_UNDEFINED"] = "PROOF_INPUT_UNDEFINED";
    ProofErrorCode["PROOF_GENERATION_FAILED"] = "PROOF_GENERATION_FAILED";
})(ProofErrorCode || (exports.ProofErrorCode = ProofErrorCode = {}));
var MerkleTreeErrorCode;
(function (MerkleTreeErrorCode) {
    MerkleTreeErrorCode["MERKLE_TREE_NOT_INITIALIZED"] = "MERKLE_TREE_NOT_INITIALIZED";
    MerkleTreeErrorCode["SOL_MERKLE_TREE_UNDEFINED"] = "SOL_MERKLE_TREE_UNDEFINED";
    MerkleTreeErrorCode["MERKLE_TREE_UNDEFINED"] = "MERKLE_TREE_UNDEFINED";
    MerkleTreeErrorCode["INPUT_UTXO_NOT_INSERTED_IN_MERKLE_TREE"] = "INPUT_UTXO_NOT_INSERTED_IN_MERKLE_TREE";
    MerkleTreeErrorCode["MERKLE_TREE_INDEX_UNDEFINED"] = "MERKLE_TREE_INDEX_UNDEFINED";
    MerkleTreeErrorCode["MERKLE_TREE_SET_SPACE_UNDEFINED"] = "MERKLE_TREE_SET_SPACE_UNDEFINED";
})(MerkleTreeErrorCode || (exports.MerkleTreeErrorCode = MerkleTreeErrorCode = {}));
var UtilsErrorCode;
(function (UtilsErrorCode) {
    UtilsErrorCode["ACCOUNT_NAME_UNDEFINED_IN_IDL"] = "ACCOUNT_NAME_UNDEFINED_IN_IDL";
    UtilsErrorCode["PROPERTY_UNDEFINED"] = "PROPERTY_UNDEFINED";
    UtilsErrorCode["LOOK_UP_TABLE_CREATION_FAILED"] = "LOOK_UP_TABLE_CREATION_FAILED";
    UtilsErrorCode["UNSUPPORTED_ARCHITECTURE"] = "UNSUPPORTED_ARCHITECTURE";
    UtilsErrorCode["UNSUPPORTED_PLATFORM"] = "UNSUPPORTED_PLATFORM";
    UtilsErrorCode["ACCOUNTS_UNDEFINED"] = "ACCOUNTS_UNDEFINED";
    UtilsErrorCode["INVALID_NUMBER"] = "INVALID_NUMBER";
})(UtilsErrorCode || (exports.UtilsErrorCode = UtilsErrorCode = {}));
var MetaError = /** @class */ (function (_super) {
    __extends(MetaError, _super);
    function MetaError(code, functionName, codeMessage) {
        var _this = _super.call(this, "".concat(code, ": ").concat(codeMessage)) || this;
        _this.code = code;
        _this.functionName = functionName;
        _this.codeMessage = codeMessage;
        return _this;
    }
    return MetaError;
}(Error));
var UtxoError = /** @class */ (function (_super) {
    __extends(UtxoError, _super);
    function UtxoError() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return UtxoError;
}(MetaError));
exports.UtxoError = UtxoError;
var SelectInUtxosError = /** @class */ (function (_super) {
    __extends(SelectInUtxosError, _super);
    function SelectInUtxosError() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return SelectInUtxosError;
}(MetaError));
exports.SelectInUtxosError = SelectInUtxosError;
var CreateUtxoError = /** @class */ (function (_super) {
    __extends(CreateUtxoError, _super);
    function CreateUtxoError() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return CreateUtxoError;
}(MetaError));
exports.CreateUtxoError = CreateUtxoError;
var RpcError = /** @class */ (function (_super) {
    __extends(RpcError, _super);
    function RpcError() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return RpcError;
}(MetaError));
exports.RpcError = RpcError;
var LookupTableError = /** @class */ (function (_super) {
    __extends(LookupTableError, _super);
    function LookupTableError() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return LookupTableError;
}(MetaError));
exports.LookupTableError = LookupTableError;
var HashError = /** @class */ (function (_super) {
    __extends(HashError, _super);
    function HashError() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return HashError;
}(MetaError));
exports.HashError = HashError;
var ProofError = /** @class */ (function (_super) {
    __extends(ProofError, _super);
    function ProofError() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return ProofError;
}(MetaError));
exports.ProofError = ProofError;
var MerkleTreeError = /** @class */ (function (_super) {
    __extends(MerkleTreeError, _super);
    function MerkleTreeError() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return MerkleTreeError;
}(MetaError));
exports.MerkleTreeError = MerkleTreeError;
var UtilsError = /** @class */ (function (_super) {
    __extends(UtilsError, _super);
    function UtilsError() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return UtilsError;
}(MetaError));
exports.UtilsError = UtilsError;

"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Token = exports.Action = void 0;
var Action;
(function (Action) {
    Action[Action["WITHDRAWAL"] = 0] = "WITHDRAWAL";
    Action[Action["DEPOSIT"] = 1] = "DEPOSIT";
})(Action = exports.Action || (exports.Action = {}));
var Token;
(function (Token) {
    Token[Token["SOL"] = 0] = "SOL";
})(Token = exports.Token || (exports.Token = {}));

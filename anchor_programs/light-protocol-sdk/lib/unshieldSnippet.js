"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.unshieldSnippet = void 0;
const constants_1 = require("./constants");
const enums_1 = require("./enums");
const errors_1 = require("./errors");
const checkAddress_1 = require("./checkAddress");
const unshieldSnippet = (recipient, amount, handle) => __awaiter(void 0, void 0, void 0, function* () {
    if (!(0, checkAddress_1.checkAddress)(recipient))
        return errors_1.invalidAddressError; // TODO - test checker
    if (amount <= 0)
        return errors_1.insufficientAmountError;
    const token = enums_1.Token.SOL;
    const left = 0;
    const top = 0;
    const width = 500;
    const height = 700;
    const positionAndSize = `left=${left},top=${top},width=${width},height=${height},popup,resizable,scrollbars,titlebar=0,toolbar=0,location=0,status=0`;
    const popupWindow = window.open(`${constants_1.LIGHTSHIELD_WIDGET}?handle=${handle}&token=${enums_1.Token[token]}&recipient=${recipient}&amount=${amount}`, '_blank', positionAndSize);
    if (popupWindow == null) {
        return errors_1.popupError;
    }
    let result = yield new Promise((resolve, reject) => {
        window.onmessage = (event) => {
            if (event.origin !== 'http://localhost:3000') {
                console.log('WRONG ORIGIN');
                console.log('ORIGIN', event.origin);
                // return;
                reject('WRONG ORIGIN');
            }
            else {
                console.log('correct origin!');
                console.log('EVENT?', event);
                console.log('data', event.data);
                if (event.data.recipient === recipient &&
                    Number(event.data.amount) === Number(amount)) {
                    console.log('CORRECT VERIFICATION HASH');
                    if (event.data.unshieldSuccess) {
                        console.log('SUCCESSFUL UNSHIELDING');
                        // window.removeEventListener("message", unshieldSuccessListener)
                        popupWindow.close();
                        resolve(true);
                        // Handle success
                    }
                    else {
                        // Handle failure
                        reject('UNSHIELDING DIDNT WORK');
                    }
                }
                else {
                    reject('INCORRECT AMOUNT / RECIPIENT');
                    // return;
                }
            }
        };
    });
    return result;
});
exports.unshieldSnippet = unshieldSnippet;

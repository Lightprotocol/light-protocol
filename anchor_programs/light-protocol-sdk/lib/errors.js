"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.insufficientAmountError = exports.invalidAddressError = exports.popupError = void 0;
exports.popupError = {
    code: 'WINDOW_POPUP_ERROR',
    message: 'There was an error opening the PopupWindow',
};
exports.invalidAddressError = {
    code: 'INVALID_ADDRESS_ERROR',
    message: 'The provided address is not valid',
};
exports.insufficientAmountError = {
    code: 'INSUFFICIENT_AMOUNT_ERROR',
    message: 'The amount has to be greater than 0',
};

"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.decodedPrivkey = void 0;
const decodedPrivkey = (privkeyBytes) => {
    const decodedPrivkey = new Uint8Array(64);
    privkeyBytes
        .split(',')
        .map((b, index) => (decodedPrivkey[index] = parseInt(b)));
    return decodedPrivkey;
};
exports.decodedPrivkey = decodedPrivkey;

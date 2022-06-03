"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.toUintArray = void 0;
// @ts-nocheck
const toUintArray = function (value) {
    let buffer;
    console.log(value);
    if (typeof value !== Buffer) {
        buffer = Buffer.from(Object.values(value));
    }
    else if (typeof value === Uint8Array) {
        buffer = value;
        return value;
    }
    else {
        buffer = Buffer.from(value);
    }
    const asArr = new Uint8Array(buffer.length);
    asArr.set(buffer);
    return asArr;
};
exports.toUintArray = toUintArray;

"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.leInt2Buffer = void 0;
var ffjavascript = require('ffjavascript');
const { unstringifyBigInts, leInt2Buff, beInt2Buff } = ffjavascript.utils;
const leInt2Buffer = (data, bytes = 32) => leInt2Buff(unstringifyBigInts(data), bytes);
exports.leInt2Buffer = leInt2Buffer;

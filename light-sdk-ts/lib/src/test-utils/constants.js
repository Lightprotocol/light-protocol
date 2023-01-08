"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.feeRecipient1Privkey = exports.feeRecipientPrivkey = exports.bidderPrivkey = exports.offerAuthorityPrivkey = exports.offerBurnerPrivkey = void 0;
const anchor_1 = require("@project-serum/anchor");
exports.offerBurnerPrivkey = new anchor_1.BN(Buffer.from([237, 38, 4, 149, 204, 158, 213, 144, 83, 16, 191, 94, 68, 105, 127, 237, 66, 153, 151, 133, 243, 205, 71, 208, 223, 215, 88, 171, 195, 137, 213, 159]));
exports.offerAuthorityPrivkey = new anchor_1.BN(Buffer.from([42, 150, 26, 78, 235, 21, 149, 49, 116, 215, 34, 161, 176, 34, 26, 16, 147, 30, 65, 203, 209, 221, 162, 85, 222, 209, 157, 53, 59, 28, 2, 253]));
exports.bidderPrivkey = new anchor_1.BN(Buffer.from([226, 46, 122, 35, 45, 227, 24, 192, 49, 124, 94, 100, 208, 151, 189, 53, 30, 52, 105, 191, 89, 191, 222, 95, 10, 224, 189, 95, 68, 242, 176, 69]));
exports.feeRecipientPrivkey = new anchor_1.BN(Buffer.from([133, 230, 53, 81, 251, 24, 235, 119, 96, 245, 17, 219, 97, 119, 70, 122, 172, 42, 13, 100, 232, 118, 151, 231, 230, 195, 140, 146, 220, 191, 89, 100]));
exports.feeRecipient1Privkey = new anchor_1.BN(Buffer.from([77, 196, 121, 16, 173, 229, 13, 169, 130, 94, 253, 3, 131, 221, 238, 47, 210, 118, 92, 100, 150, 225, 205, 67, 196, 129, 33, 132, 118, 54, 216, 209]));

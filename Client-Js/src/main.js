#!/usr/bin/env node

const solana = require("@solana/web3.js");

require("dotenv").config();
var rust = require("./webassembly");

const { bigInt } = require("snarkjs");

const crypto = require("crypto");

const program = require("commander");
const { UTF8 } = require("buffer-layout");

const solanaRPC = "http://localhost:8899"; //
/** BigNumber to hex string of specified length */
const toHex = (number, length = 32) =>
  "0x" + (number instanceof Buffer ? number.toString("hex") : ""); // buffer has own implementation of that...

// TODO: Determine instruction order solely on-chain. Then we can pass all non-payload tx in one block.
const instruction_order_verify_part_1 = [
  // - 0 : [u8;1477]
  3, 17, 4, 5, 20, 7, 8, 9, 18, 10, 21, 7, 8, 9, 18, 10, 22, 7, 8, 9, 18, 10,
  23, 7, 8, 9, 18, 10, 24, 7, 8, 9, 18, 10, 25, 7, 8, 9, 18, 10, 3, 17, 4, 5,
  26, 7, 8, 9, 18, 10, 27, 7, 8, 9, 18, 10, 28, 7, 8, 9, 18, 10, 3, 17, 4, 5,
  29, 7, 8, 9, 18, 10, 30, 7, 8, 9, 18, 10, 31, 7, 8, 9, 18, 10, 32, 7, 8, 9,
  18, 10, 33, 7, 8, 9, 18, 10, 34, 7, 8, 9, 18, 10, 3, 17, 4, 5, 35, 7, 8, 9,
  18, 10, 36, 7, 8, 9, 18, 10, 37, 7, 8, 9, 18, 10, 3, 17, 4, 5, 38, 7, 8, 9,
  18, 10, 39, 7, 8, 9, 18, 10, 40, 7, 8, 9, 18, 10, 3, 17, 4, 5, 41, 7, 8, 9,
  18, 10, 42, 7, 8, 9, 18, 10, 43, 7, 8, 9, 18, 10, 44, 7, 8, 9, 18, 10, 45, 7,
  8, 9, 18, 10, 46, 7, 8, 9, 18, 10, 3, 17, 4, 5, 47, 7, 8, 9, 18, 10, 48, 7, 8,
  9, 18, 10, 49, 7, 8, 9, 18, 10, 3, 17, 4, 5, 50, 7, 8, 9, 18, 10, 51, 7, 8, 9,
  18, 10, 52, 7, 8, 9, 18, 10, 3, 17, 4, 5, 53, 7, 8, 9, 18, 10, 54, 7, 8, 9,
  18, 10, 55, 7, 8, 9, 18, 10, 3, 17, 4, 5, 56, 7, 8, 9, 18, 10, 57, 7, 8, 9,
  18, 10, 58, 7, 8, 9, 18, 10, 3, 17, 4, 5, 59, 7, 8, 9, 18, 10, 60, 7, 8, 9,
  18, 10, 61, 7, 8, 9, 18, 10, 3, 17, 4, 5, 62, 7, 8, 9, 18, 10, 63, 7, 8, 9,
  18, 10, 64, 7, 8, 9, 18, 10, 3, 17, 4, 5, 65, 7, 8, 9, 18, 10, 66, 7, 8, 9,
  18, 10, 67, 7, 8, 9, 18, 10, 3, 17, 4, 5, 68, 7, 8, 9, 18, 10, 69, 7, 8, 9,
  18, 10, 70, 7, 8, 9, 18, 10, 3, 17, 4, 5, 71, 7, 8, 9, 18, 10, 72, 7, 8, 9,
  18, 10, 73, 7, 8, 9, 18, 10, 74, 7, 8, 9, 18, 10, 75, 7, 8, 9, 18, 10, 76, 7,
  8, 9, 18, 10, 3, 17, 4, 5, 77, 7, 8, 9, 18, 10, 78, 7, 8, 9, 18, 10, 79, 7, 8,
  9, 18, 10, 3, 17, 4, 5, 80, 7, 8, 9, 18, 10, 81, 7, 8, 9, 18, 10, 82, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 83, 7, 8, 9, 18, 10, 84, 7, 8, 9, 18, 10, 85, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 86, 7, 8, 9, 18, 10, 87, 7, 8, 9, 18, 10, 88, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 89, 7, 8, 9, 18, 10, 90, 7, 8, 9, 18, 10, 91, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 92, 7, 8, 9, 18, 10, 93, 7, 8, 9, 18, 10, 94, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 95, 7, 8, 9, 18, 10, 96, 7, 8, 9, 18, 10, 97, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 98, 7, 8, 9, 18, 10, 99, 7, 8, 9, 18, 10, 100, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 101, 7, 8, 9, 18, 10, 102, 7, 8, 9, 18, 10, 103, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 104, 7, 8, 9, 18, 10, 105, 7, 8, 9, 18, 10, 106, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 107, 7, 8, 9, 18, 10, 108, 7, 8, 9, 18, 10, 109, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 110, 7, 8, 9, 18, 10, 111, 7, 8, 9, 18, 10, 112, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 113, 7, 8, 9, 18, 10, 114, 7, 8, 9, 18, 10, 115, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 116, 7, 8, 9, 18, 10, 117, 7, 8, 9, 18, 10, 118, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 119, 7, 8, 9, 18, 10, 120, 7, 8, 9, 18, 10, 121, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 122, 7, 8, 9, 18, 10, 123, 7, 8, 9, 18, 10, 124, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 125, 7, 8, 9, 18, 10, 126, 7, 8, 9, 18, 10, 127, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 128, 7, 8, 9, 18, 10, 129, 7, 8, 9, 18, 10, 130, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 131, 7, 8, 9, 18, 10, 132, 7, 8, 9, 18, 10, 133, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 134, 7, 8, 9, 18, 10, 135, 7, 8, 9, 18, 10, 136, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 137, 7, 8, 9, 18, 10, 138, 7, 8, 9, 18, 10, 139, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 140, 7, 8, 9, 18, 10, 141, 7, 8, 9, 18, 10, 142, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 143, 7, 8, 9, 18, 10, 144, 7, 8, 9, 18, 10, 145, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 146, 7, 8, 9, 18, 10, 147, 7, 8, 9, 18, 10, 148, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 149, 7, 8, 9, 18, 10, 150, 7, 8, 9, 18, 10, 151, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 152, 7, 8, 9, 18, 10, 153, 7, 8, 9, 18, 10, 154, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 155, 7, 8, 9, 18, 10, 156, 7, 8, 9, 18, 10, 157, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 158, 7, 8, 9, 18, 10, 159, 7, 8, 9, 18, 10, 160, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 161, 7, 8, 9, 18, 10, 162, 7, 8, 9, 18, 10, 163, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 164, 7, 8, 9, 18, 10, 165, 7, 8, 9, 18, 10, 166, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 167, 7, 8, 9, 18, 10, 168, 7, 8, 9, 18, 10, 169, 7, 8, 9,
  18, 10, 3, 17, 4, 5, 170, 7, 8, 9, 18, 10, 171, 7, 8, 9, 18, 10, 172, 7, 8, 9,
  18, 10, 173, 7, 8, 9, 18, 10, 174, 7, 8, 9, 18, 10, 175, 7, 8, 9, 18, 10, 3,
  17, 4, 5, 176, 7, 8, 9, 18, 10, 177, 7, 8, 9, 18, 10, 178, 7, 8, 9, 18, 10, 3,
  17, 4, 5, 179, 7, 8, 9, 18, 10, 180, 7, 8, 9, 18, 10, 181, 7, 8, 9, 18, 10, 3,
  17, 4, 5, 182, 7, 8, 9, 18, 10, 183, 7, 8, 9, 18, 10, 184, 7, 8, 9, 18, 10, 3,
  17, 4, 5, 185, 7, 8, 9, 18, 10, 186, 7, 8, 9, 18, 10, 187, 7, 8, 9, 18, 10, 3,
  17, 4, 5, 188, 7, 8, 9, 18, 10, 189, 7, 8, 9, 18, 10, 190, 7, 8, 9, 18, 10, 3,
  17, 4, 5, 191, 7, 8, 9, 18, 10, 192, 7, 8, 9, 18, 10, 193, 7, 8, 9, 18, 10, 3,
  17, 4, 5, 194, 7, 8, 9, 18, 10, 195, 7, 8, 9, 18, 10, 196, 7, 8, 9, 18, 10, 3,
  17, 4, 5, 197, 7, 8, 9, 18, 10, 198, 7, 8, 9, 18, 10, 199, 7, 8, 9, 18, 10, 3,
  17, 4, 5, 200, 7, 8, 9, 18, 10, 201, 7, 8, 9, 18, 10, 202, 7, 8, 9, 18, 10, 3,
  17, 4, 5, 203, 7, 8, 9, 18, 10, 204, 7, 8, 9, 18, 10, 205, 7, 8, 9, 18, 10, 3,
  17, 4, 5, 206, 7, 8, 9, 18, 10, 207, 7, 8, 9, 18, 10, 208, 7, 8, 9, 18, 10, 3,
  17, 4, 5, 209, 7, 8, 9, 18, 10, 210, 7, 8, 9, 18, 10, 211, 7, 8, 9, 18, 10, 3,
  17, 4, 5, 212, 7, 8, 9, 18, 10, 213, 7, 8, 9, 18, 10, 214, 7, 8, 9, 18, 10, 3,
  17, 4, 5, 215, 7, 8, 9, 18, 10, 216, 7, 8, 9, 18, 10, 217, 7, 8, 9, 18, 10, 3,
  17, 4, 5, 218, 7, 8, 9, 18, 10, 219, 7, 8, 9, 18, 10, 220, 7, 8, 9, 18, 10, 3,
  17, 4, 5, 221, 7, 8, 9, 18, 10, 222, 7, 8, 9, 18, 10, 223, 7, 8, 9, 18, 10,
  16,
];

const instruction_order_verify_part_2 = [
  0, 1, 2, 3, 4, 5, 120, 6, 7, 101, 102, 8, 9, 10, 11, 104, 12, 13, 14, 15, 8,
  9, 10, 11, 104, 12, 16, 17, 18, 19, 20, 105, 21, 22, 20, 105, 21, 22, 28, 29,
  30, 31, 107, 32, 20, 105, 21, 22, 20, 105, 21, 22, 23, 24, 25, 26, 106, 27,
  20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 23, 24, 25, 26, 106, 27,
  20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105,
  21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22,
  23, 24, 25, 26, 106, 27, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22,
  20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105,
  21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22,
  20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105,
  21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22,
  20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105,
  21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22,
  20, 105, 21, 22, 20, 105, 21, 22, 23, 24, 25, 26, 106, 27, 20, 105, 21, 22,
  20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105,
  21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22,
  20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105,
  21, 22, 20, 105, 21, 22, 33, 34, 35, 36, 37, 38, 39, 108, 40, 41, 18, 42, 43,
  109, 44, 45, 43, 109, 44, 45, 51, 52, 53, 54, 111, 55, 43, 109, 44, 45, 43,
  109, 44, 45, 46, 47, 48, 49, 110, 50, 43, 109, 44, 45, 43, 109, 44, 45, 43,
  109, 44, 45, 46, 47, 48, 49, 110, 50, 43, 109, 44, 45, 43, 109, 44, 45, 43,
  109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44,
  45, 43, 109, 44, 45, 43, 109, 44, 45, 46, 47, 48, 49, 110, 50, 43, 109, 44,
  45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43,
  109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44,
  45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43,
  109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44,
  45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43,
  109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44,
  45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 46,
  47, 48, 49, 110, 50, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43,
  109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44,
  45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43,
  109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 56, 57, 36,
  37, 38, 39, 108, 40, 41, 18, 42, 43, 109, 44, 45, 43, 109, 44, 45, 51, 52, 53,
  54, 111, 55, 43, 109, 44, 45, 43, 109, 44, 45, 46, 47, 48, 49, 110, 50, 43,
  109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 46, 47, 48, 49, 110, 50, 43,
  109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44,
  45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 46,
  47, 48, 49, 110, 50, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43,
  109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44,
  45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43,
  109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44,
  45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43,
  109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44,
  45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43,
  109, 44, 45, 43, 109, 44, 45, 46, 47, 48, 49, 110, 50, 43, 109, 44, 45, 43,
  109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44,
  45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43,
  109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44,
  45, 43, 109, 44, 45, 56, 58, 59, 36, 37, 38, 39, 108, 40, 61, 62, 63, 64, 112,
  65, 41, 18, 66, 67, 113, 68, 69, 67, 113, 68, 69, 75, 76, 77, 78, 115, 79, 67,
  113, 68, 69, 67, 113, 68, 69, 70, 71, 72, 73, 114, 74, 67, 113, 68, 69, 67,
  113, 68, 69, 67, 113, 68, 69, 70, 71, 72, 73, 114, 74, 67, 113, 68, 69, 67,
  113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68,
  69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 70, 71, 72, 73, 114,
  74, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67,
  113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68,
  69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67,
  113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68,
  69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67,
  113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68,
  69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67,
  113, 68, 69, 70, 71, 72, 73, 114, 74, 67, 113, 68, 69, 67, 113, 68, 69, 67,
  113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68,
  69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67,
  113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68,
  69, 80, 81, 18, 82, 43, 109, 44, 45, 43, 109, 44, 45, 51, 52, 53, 54, 111, 55,
  43, 109, 44, 45, 43, 109, 44, 45, 83, 84, 85, 86, 116, 87, 43, 109, 44, 45,
  43, 109, 44, 45, 43, 109, 44, 45, 83, 84, 85, 86, 116, 87, 43, 109, 44, 45,
  43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109,
  44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 83, 84, 85, 86,
  116, 87, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45,
  43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109,
  44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45,
  43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109,
  44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45,
  43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109,
  44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45,
  43, 109, 44, 45, 83, 84, 85, 86, 116, 87, 43, 109, 44, 45, 43, 109, 44, 45,
  43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109,
  44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45,
  43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109,
  44, 45, 56, 88, 89, 90, 57, 36, 37, 38, 39, 108, 40, 91, 92, 93, 94, 117, 95,
  96, 97, 98, 99, 118, 100, 121, 122, 123, 124, 103,
];

const INSTRUCTION_ARRAY_INSERT_MERKLE_TREE_HEIGHT_2 = [
  24, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30,
  31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13,
  14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 26,
];
const INSTRUCTION_ARRAY_INSERT_MERKLE_TREE_HEIGHT_18 = [
  24, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30,
  31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13,
  14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6,
  7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23,
  25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31,
  32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
  15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7,
  8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25,
  27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32,
  33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
  16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9,
  10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1,
  2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33,
  19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
  17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
  11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2,
  3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19,
  20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
  18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
  12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3,
  4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20,
  21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
  30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
  13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 26,
];

let INSTRUCTION_ARRAY_INSERT_MERKLE_TREE_HEIGHT_11 = [
  24, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30,
  31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13,
  14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6,
  7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23,
  25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31,
  32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
  15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7,
  8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25,
  27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32,
  33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
  16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9,
  10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1,
  2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33,
  19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
  17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 26,
];

const PRIVATE_KEY = process.env.PRIVATE_KEY;
const prep_inputs_program_id = process.env.PROGRAM_PREPARE_INPUTS_ID;
const m_expo_program_id = process.env.PROGRAM_ID;

// merkle tree program accounts
const storage_account_pkey = "7ynT1UdHy2tjDJvYaz4DP5TXiengm9q7HjBobKJ2iU9b";
const merkle_tree_storage_acc_pkey =
  "FxbvBBSSPv14hFtpck6umSNBEEVPDj2RaX9TLZNeeiFn";

var connection;

function sleep(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

// Connection check
async function getNodeConnection(url) {
  connection = new solana.Connection(url, "recent");
  const version = await connection.getVersion();
  console.log("Connection to cluster established:", url, version);
}

async function tryTransaction(transaction, account, connection) {
  var success = false;
  var i = 0;
  while (!success && i < 2) {
    try {
      //await setTimeout(function(){ console.log("retrying"); }, 5000);
      await sleep(500);
      transaction.recentBlockhash = await connection.getRecentBlockhash();

      tx_hash = await solana.sendAndConfirmTransaction(
        connection,
        transaction,
        [account],
        {
          commitment: "singleGossip",
          preflightCommitment: "singleGossip",
        }
      );
      success = true;
      return tx_hash;
    } catch (e) {
      console.log(e);
      i++;
    }
  }
  if (!success && i == 10) {
    throw new Error("tx failed");
  }
}

async function create_program_acc(from_acc, seed, storage_space, program_id) {
  //var lamports = 1 * 10 ** 10;
  const program_acc = new solana.PublicKey(program_id);
  var inst_arr = new Uint8Array(Buffer.from(seed));

  const storage_account = await solana.PublicKey.createWithSeed(
    from_acc.publicKey,
    seed,
    program_acc
  );

  var acc_params = {
    fromPubkey: from_acc.publicKey,
    newAccountPubkey: storage_account,
    basePubkey: from_acc.publicKey,
    seed: seed,
    //enough rent for one epoch
    lamports: await connection.getMinimumBalanceForRentExemption(storage_space), // / (365),
    space: storage_space,
    programId: program_acc,
  };

  const tx = new solana.Transaction().add(
    solana.SystemProgram.createAccountWithSeed(acc_params)
  );

  let x = await tryTransaction(tx, from_acc, connection);
  console.log("storage account creation txid: ", x);
  console.log("storage account pubkey = ", storage_account.toBase58());

  return storage_account.toBase58();
}

async function build_merkle_tree() {
  // Creating Transaction Instructions
  const privateKeyDecoded = PRIVATE_KEY.split(",").map((s) => parseInt(s));
  const account = new solana.Account(privateKeyDecoded);

  //init transaction parsing an inited merkle_tree into the account
  let tx1 = new solana.Transaction();

  var instruction_id = new Uint8Array(3);
  instruction_id[0] = 240;
  instruction_id[1] = 0;
  instruction_id[2] = 1;

  instruction_id = Buffer.from(instruction_id.buffer);

  tx1.add(
    new solana.TransactionInstruction({
      programId: m_expo_program_id,
      keys: [
        { pubkey: account.publicKey, isSigner: true, isWritable: false },
        {
          pubkey: merkle_tree_storage_acc_pkey,
          isSigner: false,
          isWritable: true,
        },
      ],
      data: instruction_id,
    })
  );
  tx1.recentBlockhash = await connection.getRecentBlockhash();

  let x = await tryTransaction(tx1, account, connection);
  console.log("Call tx id ", x);
}

async function insert_into_merkle_tree(hash_tmp_account_pkey, note, amount) {
  // Creating Transaction Instructions
  const privateKeyDecoded = PRIVATE_KEY.split(",").map((s) => parseInt(s));
  const account = new solana.Account(privateKeyDecoded);
  var storage_acc = new solana.PublicKey(storage_account_pkey);
  let params = {
    fromPubkey: account.publicKey,
    toPubkey: storage_account_pkey,
  };

  let leaf_hash;

  //disposible transfer account owned by the contract to check deposited amount
  var seed1 = crypto.randomBytes(15).toString("hex");
  let tmp_tranfer_account = await create_program_acc(
    account,
    seed1,
    0,
    m_expo_program_id
  );

  let tx1 = new solana.Transaction();
  tx1.add(
    await solana.SystemProgram.transfer({
      fromPubkey: account.publicKey,
      toPubkey: tmp_tranfer_account,
      lamports: 1e9, //TODO: replace with amount
    })
  );
  //trigger deposit instruction
  var instruction_id = new Uint8Array(3);
  instruction_id[0] = 34;
  instruction_id[1] = 0;
  instruction_id[2] = 1;

  instruction_id = Buffer.from(instruction_id.buffer);

  tx1.add(
    new solana.TransactionInstruction({
      programId: m_expo_program_id,
      keys: [
        { pubkey: account.publicKey, isSigner: true, isWritable: false },
        { pubkey: hash_tmp_account_pkey, isSigner: false, isWritable: true },
        {
          pubkey: merkle_tree_storage_acc_pkey,
          isSigner: false,
          isWritable: true,
        },
        { pubkey: tmp_tranfer_account, isSigner: false, isWritable: true },
      ],
      data: instruction_id,
    })
  );
  var instruction_id_if = new Uint8Array(35);
  leaf_hash = Buffer.from(note);
  for (let i = 2; i < 34; i++) {
    instruction_id_if[i] = leaf_hash[i - 2];
  }
  instruction_id_if[1] = 0;
  instruction_id_if[34] = 0;
  instruction_id_if[0] = 1;
  instruction_id_if = Buffer.from(instruction_id_if.buffer);
  tx1.add(
    new solana.TransactionInstruction({
      programId: m_expo_program_id,
      keys: [
        { pubkey: account.publicKey, isSigner: true, isWritable: false },
        { pubkey: hash_tmp_account_pkey, isSigner: false, isWritable: true },
        {
          pubkey: merkle_tree_storage_acc_pkey,
          isSigner: false,
          isWritable: true,
        },
      ],
      data: instruction_id_if,
    })
  );

  let x = await tryTransaction(tx1, account, connection);

  console.log("Sol transfer txid ", x);

  //creating an instruction for every entry in the instruction order array,
  //adding these instructions to a transaction and sending it
  //instructions to absorb
  let tx = new solana.Transaction();

  for (
    let i = 0;
    i < INSTRUCTION_ARRAY_INSERT_MERKLE_TREE_HEIGHT_11.length;
    i++
  ) {
    let tx = new solana.Transaction();

    //j is the number of instructions in one transactions
    for (
      let j = 0;
      j < 100 && j + i < INSTRUCTION_ARRAY_INSERT_MERKLE_TREE_HEIGHT_11.length;
      j++
    ) {
      //console.log("creating instruction: ", instruction_order[j + i]);
      if (INSTRUCTION_ARRAY_INSERT_MERKLE_TREE_HEIGHT_11[j + i] == 24) {
        //is added before should be in the same as the monetary deposit for security
      } else {
        var instruction_id_else = new Uint8Array(3);

        instruction_id_else[1] = 0;
        instruction_id_else[2] = 0;
        //changed instructions to onchain instruction index
        instruction_id_else[0] = 1; //INSTRUCTION_ARRAY_INSERT_MERKLE_TREE_HEIGHT_11[j + i];
        instruction_id_else = Buffer.from(instruction_id_else.buffer);
        tx.add(
          new solana.TransactionInstruction({
            programId: m_expo_program_id,
            keys: [
              { pubkey: account.publicKey, isSigner: true, isWritable: false },
              {
                pubkey: hash_tmp_account_pkey,
                isSigner: false,
                isWritable: true,
              },
              {
                pubkey: merkle_tree_storage_acc_pkey,
                isSigner: false,
                isWritable: true,
              },
            ],
            data: instruction_id_else,
          })
        );
      }
    }
    //should be one less than intended since i already increases by 1
    i += 99;

    let x_intern = await tryTransaction(tx, account, connection);

    console.log(` ${i + 1} instr txid ${x_intern}`);
  }

  return leaf_hash;
}

async function fill_p(tmp_account_prepare_inputs, inputs_bytes) {
  const privateKeyDecoded = PRIVATE_KEY.split(",").map((s) => parseInt(s));
  const account = new solana.Account(privateKeyDecoded);

  let complete_instruction_order_fill_p = "";
  // P2 onchain

  //readAcc_Miller(tmp_account_prepare_inputs)
  // init gic
  let tx_init = new solana.Transaction();
  var instruction_id = new Uint8Array(210);
  instruction_id[1] = 3;
  instruction_id[0] = 0;
  inputs_bytes.map(
    (x, index) => (instruction_id[index + 2] = inputs_bytes[index])
  );
  instruction_id = Buffer.from(instruction_id.buffer);
  complete_instruction_order_fill_p += instruction_id[0];
  tx_init.add(
    new solana.TransactionInstruction({
      programId: prep_inputs_program_id,
      keys: [
        { pubkey: account.publicKey, isSigner: true, isWritable: false },
        {
          pubkey: tmp_account_prepare_inputs,
          isSigner: false,
          isWritable: true,
        },
        {
          pubkey: merkle_tree_storage_acc_pkey,
          isSigner: false,
          isWritable: true,
        },
      ],
      data: instruction_id,
    })
  );

  for (let i = 0; i < 51; i++) {
    var instruction_id = new Uint8Array(2);
    instruction_id[1] = 3;
    instruction_id[0] = 0;
    tx_init.add(
      new solana.TransactionInstruction({
        programId: prep_inputs_program_id,
        keys: [
          { pubkey: account.publicKey, isSigner: true, isWritable: false },
          {
            pubkey: tmp_account_prepare_inputs,
            isSigner: false,
            isWritable: true,
          },
        ],
        data: instruction_id,
      })
    );
  }

  let x3 = await tryTransaction(tx_init, account, connection);

  console.log("Call tx init id ", x3);

  for (let k = 0; k < 4; k++) {
    // 1 new res
    let tx = new solana.Transaction();
    var instruction_id = new Uint8Array(2);
    instruction_id[1] = 3;
    instruction_id[0] = 1;
    instruction_id = Buffer.from(instruction_id.buffer);
    complete_instruction_order_fill_p += ", " + instruction_id[0];
    tx.add(
      new solana.TransactionInstruction({
        programId: prep_inputs_program_id,
        keys: [
          { pubkey: account.publicKey, isSigner: true, isWritable: false },
          {
            pubkey: tmp_account_prepare_inputs,
            isSigner: false,
            isWritable: true,
          },
        ],
        data: instruction_id,
      })
    );
    let x_intern = await tryTransaction(tx, account, connection);

    console.log(` ${1} new res - instr tx id ${x_intern}`);

    for (let i = 0; i < 256; i++) {
      let tx = new solana.Transaction();

      for (let j = 0; j < 100 && j + i < 256; j++) {
        var instruction_id = new Uint8Array(3);
        instruction_id[2] = j + i; // current_index 0-255 / 0..256
        instruction_id[1] = 3;
        instruction_id[0] = k + 2; // 0+2 to 3+2
        instruction_id = Buffer.from(instruction_id.buffer);
        complete_instruction_order_fill_p += ", " + instruction_id[0];
        tx.add(
          new solana.TransactionInstruction({
            programId: prep_inputs_program_id,
            keys: [
              { pubkey: account.publicKey, isSigner: true, isWritable: false },
              {
                pubkey: tmp_account_prepare_inputs,
                isSigner: false,
                isWritable: true,
              },
            ],
            data: instruction_id,
          })
        );
      }
      i += 99;

      let x4 = await tryTransaction(tx, account, connection);

      console.log("Call tx2 id ", x4);
    }

    // 6 write res to g_ic
    let tx6 = new solana.Transaction();
    var instruction_id = new Uint8Array(2);
    instruction_id[1] = 3;
    instruction_id[0] = 6;
    instruction_id = Buffer.from(instruction_id.buffer);
    complete_instruction_order_fill_p += ", " + instruction_id[0];
    tx6.add(
      new solana.TransactionInstruction({
        programId: prep_inputs_program_id,
        keys: [
          { pubkey: account.publicKey, isSigner: true, isWritable: false },
          {
            pubkey: tmp_account_prepare_inputs,
            isSigner: false,
            isWritable: true,
          },
        ],
        data: instruction_id,
      })
    );
    let x_intern_1 = await tryTransaction(tx6, account, connection);

    console.log(` ${6} instr tx id ${x_intern_1}`);
  }

  // 7,8 prepare final g_ic in account_prepare_inputs
  let tx_g_ic = new solana.Transaction();
  let instructions = [7, 8]; // based on split
  for (instruction in instructions) {
    var instruction_id = new Uint8Array(2);
    instruction_id[1] = 3;
    instruction_id[0] = instructions[instruction];
    instruction_id = Buffer.from(instruction_id.buffer);
    complete_instruction_order_fill_p += ", " + instruction_id[0];
    tx_g_ic.add(
      new solana.TransactionInstruction({
        programId: prep_inputs_program_id,
        keys: [
          { pubkey: account.publicKey, isSigner: true, isWritable: false },
          {
            pubkey: tmp_account_prepare_inputs,
            isSigner: false,
            isWritable: true,
          },
        ],
        data: instruction_id,
      })
    );
  }
  let x_g_ic = await tryTransaction(tx_g_ic, account, connection);

  console.log("Call tx_g_ic 1/2 id ", x_g_ic);

  //console.log("pub const complete_instruction_order_fill_p = [ " + complete_instruction_order_fill_p + "];")

  console.assert(
    "0, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 6, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 6, 1, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 1, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 6, 7, 8" ==
      complete_instruction_order_fill_p
  );
  // note: verif1 first instruction will read final g_ic  from this program/
}

async function verify_part_1(
  tmp_account_miller_loop,
  tmp_account_prepare_inputs,
  tmp_account_final_exp,
  proof_a_bytes,
  proof_b_bytes,
  proof_c_bytes
) {
  // Creating Transaction Instructions
  const privateKeyDecoded = PRIVATE_KEY.split(",").map((s) => parseInt(s));
  const account = new solana.Account(privateKeyDecoded);
  let complete_instruction_order_verify_one = "";

  var acc_main = new solana.PublicKey(tmp_account_miller_loop);

  console.log(
    "--------------------- packing p2 into acc_main -------------------"
  );
  // -1
  // write final g_ic to miller account - 00
  let tx_g_ic_get = new solana.Transaction();
  let instructions = [251]; // based on split
  for (instruction in instructions) {
    var instruction_id = new Uint8Array(2);
    instruction_id[1] = 1;
    instruction_id[0] = instructions[instruction];
    instruction_id = Buffer.from(instruction_id.buffer);
    complete_instruction_order_verify_one += instruction_id[0];
    tx_g_ic_get.add(
      new solana.TransactionInstruction({
        programId: m_expo_program_id,
        keys: [
          { pubkey: account.publicKey, isSigner: true, isWritable: false },
          {
            pubkey: tmp_account_miller_loop,
            isSigner: false,
            isWritable: true,
          },
          {
            pubkey: tmp_account_prepare_inputs,
            isSigner: false,
            isWritable: false,
          },
        ],
        data: instruction_id,
      })
    );
  }
  let g_ic_get = await tryTransaction(tx_g_ic_get, account, connection);

  console.log("Call tx_g_ic_get id ", g_ic_get);

  //parse p into the account and init f:
  let proof_a_c_bytes = [...proof_a_bytes, ...proof_c_bytes];
  let tx2 = new solana.Transaction();
  var instruction_id = new Uint8Array(194);
  instruction_id[0] = 230;
  instruction_id[1] = 1;
  proof_a_c_bytes.map(
    (x, index) => (instruction_id[index + 2] = proof_a_c_bytes[index])
  ); // 192 bytes
  instruction_id = Buffer.from(instruction_id.buffer);
  complete_instruction_order_verify_one += ", " + instruction_id[0];
  tx2.add(
    new solana.TransactionInstruction({
      programId: m_expo_program_id,
      keys: [
        { pubkey: account.publicKey, isSigner: true, isWritable: false },
        { pubkey: tmp_account_miller_loop, isSigner: false, isWritable: true },
      ],
      data: instruction_id,
    })
  );

  // init the coeffs custom thingy (pass proof.b)
  var instruction_id = new Uint8Array(194);
  instruction_id[0] = 237;
  instruction_id[1] = 1;
  proof_b_bytes.map(
    (x, index) => (instruction_id[index + 2] = proof_b_bytes[index])
  ); // 192 bytes
  complete_instruction_order_verify_one += ", " + instruction_id[0];
  tx2.add(
    new solana.TransactionInstruction({
      programId: m_expo_program_id,
      keys: [
        { pubkey: account.publicKey, isSigner: true, isWritable: false },
        { pubkey: tmp_account_miller_loop, isSigner: false, isWritable: true },
      ],
      data: instruction_id,
    })
  );
  let x2 = await tryTransaction(tx2, account, connection);

  console.log("Call 230,194 id ", x2);

  // see if instructions are one of M1
  var one = [
    0, 3, 6, 9, 12, 15, 18, 21, 24, 27, 30, 33, 36, 39, 42, 45, 48, 51, 54, 57,
    60, 63, 66, 69, 72, 75, 78, 81, 84, 87, 90, 93, 96, 99, 102, 105, 108, 111,
    114, 117, 120, 123, 126, 129, 132, 135, 138, 141, 144, 147, 150, 153, 156,
    159, 162, 165, 168, 171, 174, 177, 180, 183, 186, 189, 192, 195, 198, 201,
  ];
  var two = [
    1, 4, 7, 10, 13, 16, 19, 22, 25, 28, 31, 34, 37, 40, 43, 46, 49, 52, 55, 58,
    61, 64, 67, 70, 73, 76, 79, 82, 85, 88, 91, 94, 97, 100, 103, 106, 109, 112,
    115, 118, 121, 124, 127, 130, 133, 136, 139, 142, 145, 148, 151, 154, 157,
    160, 163, 166, 169, 172, 175, 178, 181, 184, 187, 190, 193, 196, 199, 202,
  ];
  var three = [
    2, 5, 8, 11, 14, 17, 20, 23, 26, 29, 32, 35, 38, 41, 44, 47, 50, 53, 56, 59,
    62, 65, 68, 71, 74, 77, 80, 83, 86, 89, 92, 95, 98, 101, 104, 107, 110, 113,
    116, 119, 122, 125, 128, 131, 134, 137, 140, 143, 146, 149, 152, 155, 158,
    161, 164, 167, 170, 173, 176, 179, 182, 185, 188, 191, 194, 197, 200, 203,
  ];

  //creating an instruction for every entry in the instruction order array,
  //adding these instructions to a transaction and sending it
  let iter_r = 288;
  let counter = 0;
  for (let i = 0; i < instruction_order_verify_part_1.length; i++) {
    let tx = new solana.Transaction();

    //j is the number of instructions in one transaction
    for (
      let j = 0;
      j < 115 && j + i < instruction_order_verify_part_1.length;
      j++
    ) {
      if (
        instruction_order_verify_part_1[j + i] > 19 &&
        instruction_order_verify_part_1[j + i] < 224
      ) {
        counter = 1;
        let instruction_combo = [];
        if (one.includes(instruction_order_verify_part_1[j + i] - 20)) {
          let addition_step_custom_rounds = [3, 12, 24, 54, 153];
          if (
            addition_step_custom_rounds.includes(
              instruction_order_verify_part_1[j + i] - 20
            )
          ) {
            // the one instructions...
            instruction_combo = [234, 235, 236]; // addsteps
          } else {
            // the other instruction...
            instruction_combo = [231, 232, 233];
          }
        } else if (two.includes(instruction_order_verify_part_1[j + i] - 20)) {
          // id to get coeff from pk
          instruction_combo = [225];
        } else if (
          three.includes(instruction_order_verify_part_1[j + i] - 20)
        ) {
          // id to get coeff from other pkval
          // instruction_combo = [8]
          instruction_combo = [226];
        }
        // inject the coeff instructions if one of M1
        //readAcc_Miller(tmp_account_miller_loop);
        if (instruction_combo.length > 0) {
          for (t in instruction_combo) {
            //console.log("INSTRUCTIONCOMBO: i ", instruction_combo[t])
            var instruction_id = new Uint8Array(2);
            instruction_id[0] = instruction_combo[t];
            instruction_id[1] = 1;
            instruction_id = Buffer.from(instruction_id.buffer);
            complete_instruction_order_verify_one += ", " + instruction_id[0];
            tx.add(
              new solana.TransactionInstruction({
                programId: m_expo_program_id,
                keys: [
                  {
                    pubkey: account.publicKey,
                    isSigner: true,
                    isWritable: false,
                  },
                  {
                    pubkey: tmp_account_miller_loop,
                    isSigner: false,
                    isWritable: true,
                  },
                ],
                data: instruction_id,
              })
            );
          }

          //console.log("the added i 1 : ",instruction_order_verify_part_1[j + i], j,i, "iov1: ", instruction_order_verify_part_1)
          var instruction_id = new Uint8Array(2);
          // var instruction_id = new Uint8Array(2);
          instruction_id[0] = instruction_order_verify_part_1[j + i];
          instruction_id[1] = 1;
          instruction_id = Buffer.from(instruction_id.buffer);
          complete_instruction_order_verify_one += ", " + instruction_id[0];
          tx.add(
            new solana.TransactionInstruction({
              programId: m_expo_program_id,
              keys: [
                {
                  pubkey: account.publicKey,
                  isSigner: true,
                  isWritable: false,
                },
                {
                  pubkey: tmp_account_miller_loop,
                  isSigner: false,
                  isWritable: true,
                },
              ],
              data: instruction_id,
            })
          );
          // this desnt happen anymore (test):
        } else {
          console.log(
            "ERR: NO INSTRUCTION COMBO FOUND",
            instruction_order_verify_part_1[j + i]
          );
        }
      } else {
        // 1 normal instruction not m1
        var instruction_id = new Uint8Array(2);
        instruction_id[0] = instruction_order_verify_part_1[j + i];
        instruction_id[1] = 1;
        instruction_id = Buffer.from(instruction_id.buffer);
        complete_instruction_order_verify_one += ", " + instruction_id[0];
        tx.add(
          new solana.TransactionInstruction({
            programId: m_expo_program_id,
            keys: [
              { pubkey: account.publicKey, isSigner: true, isWritable: false },
              {
                pubkey: tmp_account_miller_loop,
                isSigner: false,
                isWritable: true,
              },
            ],
            data: instruction_id,
          })
        );
      }
    }
    //should be one less than intended since i already increases by 1 (also: ignoring the injected instructions as they are not part of the instruction array)
    i += 114;
    let x_intern = await tryTransaction(tx, account, connection);

    console.log(` ${i + 1} instr tx id ${x_intern}`);
    if (counter == 1) {
      counter = 0;
    }
  }

  //last transaction to view result of computation
  let tx_last = new solana.Transaction();
  var instruction_id = new Uint8Array(2);
  instruction_id[0] = 255;
  instruction_id[1] = 1;
  instruction_id = Buffer.from(instruction_id.buffer);
  //complete_instruction_order_verify_one += ", " + instruction_id[0];
  complete_instruction_order_verify_one += ", " + instruction_id[0];
  tx_last.add(
    new solana.TransactionInstruction({
      programId: m_expo_program_id,
      keys: [
        { pubkey: account.publicKey, isSigner: true, isWritable: false },
        { pubkey: tmp_account_miller_loop, isSigner: false, isWritable: true },
        { pubkey: tmp_account_final_exp, isSigner: false, isWritable: true },
      ],
      data: instruction_id,
    })
  );
  let res = await tryTransaction(tx_last, account, connection);

  console.log("Call tx id ", res);

  //console.log("pub const complete_instruction_order_verify_one = [ " + complete_instruction_order_verify_one + "];")
  console.assert(
    complete_instruction_order_verify_one ==
      "251, 230, 237, 3, 17, 4, 5, 231, 232, 233, 20, 7, 8, 9, 18, 10, 225, 21, 7, 8, 9, 18, 10, 226, 22, 7, 8, 9, 18, 10, 234, 235, 236, 23, 7, 8, 9, 18, 10, 225, 24, 7, 8, 9, 18, 10, 226, 25, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 26, 7, 8, 9, 18, 10, 225, 27, 7, 8, 9, 18, 10, 226, 28, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 29, 7, 8, 9, 18, 10, 225, 30, 7, 8, 9, 18, 10, 226, 31, 7, 8, 9, 18, 10, 234, 235, 236, 32, 7, 8, 9, 18, 10, 225, 33, 7, 8, 9, 18, 10, 226, 34, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 35, 7, 8, 9, 18, 10, 225, 36, 7, 8, 9, 18, 10, 226, 37, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 38, 7, 8, 9, 18, 10, 225, 39, 7, 8, 9, 18, 10, 226, 40, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 41, 7, 8, 9, 18, 10, 225, 42, 7, 8, 9, 18, 10, 226, 43, 7, 8, 9, 18, 10, 234, 235, 236, 44, 7, 8, 9, 18, 10, 225, 45, 7, 8, 9, 18, 10, 226, 46, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 47, 7, 8, 9, 18, 10, 225, 48, 7, 8, 9, 18, 10, 226, 49, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 50, 7, 8, 9, 18, 10, 225, 51, 7, 8, 9, 18, 10, 226, 52, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 53, 7, 8, 9, 18, 10, 225, 54, 7, 8, 9, 18, 10, 226, 55, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 56, 7, 8, 9, 18, 10, 225, 57, 7, 8, 9, 18, 10, 226, 58, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 59, 7, 8, 9, 18, 10, 225, 60, 7, 8, 9, 18, 10, 226, 61, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 62, 7, 8, 9, 18, 10, 225, 63, 7, 8, 9, 18, 10, 226, 64, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 65, 7, 8, 9, 18, 10, 225, 66, 7, 8, 9, 18, 10, 226, 67, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 68, 7, 8, 9, 18, 10, 225, 69, 7, 8, 9, 18, 10, 226, 70, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 71, 7, 8, 9, 18, 10, 225, 72, 7, 8, 9, 18, 10, 226, 73, 7, 8, 9, 18, 10, 234, 235, 236, 74, 7, 8, 9, 18, 10, 225, 75, 7, 8, 9, 18, 10, 226, 76, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 77, 7, 8, 9, 18, 10, 225, 78, 7, 8, 9, 18, 10, 226, 79, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 80, 7, 8, 9, 18, 10, 225, 81, 7, 8, 9, 18, 10, 226, 82, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 83, 7, 8, 9, 18, 10, 225, 84, 7, 8, 9, 18, 10, 226, 85, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 86, 7, 8, 9, 18, 10, 225, 87, 7, 8, 9, 18, 10, 226, 88, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 89, 7, 8, 9, 18, 10, 225, 90, 7, 8, 9, 18, 10, 226, 91, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 92, 7, 8, 9, 18, 10, 225, 93, 7, 8, 9, 18, 10, 226, 94, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 95, 7, 8, 9, 18, 10, 225, 96, 7, 8, 9, 18, 10, 226, 97, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 98, 7, 8, 9, 18, 10, 225, 99, 7, 8, 9, 18, 10, 226, 100, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 101, 7, 8, 9, 18, 10, 225, 102, 7, 8, 9, 18, 10, 226, 103, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 104, 7, 8, 9, 18, 10, 225, 105, 7, 8, 9, 18, 10, 226, 106, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 107, 7, 8, 9, 18, 10, 225, 108, 7, 8, 9, 18, 10, 226, 109, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 110, 7, 8, 9, 18, 10, 225, 111, 7, 8, 9, 18, 10, 226, 112, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 113, 7, 8, 9, 18, 10, 225, 114, 7, 8, 9, 18, 10, 226, 115, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 116, 7, 8, 9, 18, 10, 225, 117, 7, 8, 9, 18, 10, 226, 118, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 119, 7, 8, 9, 18, 10, 225, 120, 7, 8, 9, 18, 10, 226, 121, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 122, 7, 8, 9, 18, 10, 225, 123, 7, 8, 9, 18, 10, 226, 124, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 125, 7, 8, 9, 18, 10, 225, 126, 7, 8, 9, 18, 10, 226, 127, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 128, 7, 8, 9, 18, 10, 225, 129, 7, 8, 9, 18, 10, 226, 130, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 131, 7, 8, 9, 18, 10, 225, 132, 7, 8, 9, 18, 10, 226, 133, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 134, 7, 8, 9, 18, 10, 225, 135, 7, 8, 9, 18, 10, 226, 136, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 137, 7, 8, 9, 18, 10, 225, 138, 7, 8, 9, 18, 10, 226, 139, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 140, 7, 8, 9, 18, 10, 225, 141, 7, 8, 9, 18, 10, 226, 142, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 143, 7, 8, 9, 18, 10, 225, 144, 7, 8, 9, 18, 10, 226, 145, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 146, 7, 8, 9, 18, 10, 225, 147, 7, 8, 9, 18, 10, 226, 148, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 149, 7, 8, 9, 18, 10, 225, 150, 7, 8, 9, 18, 10, 226, 151, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 152, 7, 8, 9, 18, 10, 225, 153, 7, 8, 9, 18, 10, 226, 154, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 155, 7, 8, 9, 18, 10, 225, 156, 7, 8, 9, 18, 10, 226, 157, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 158, 7, 8, 9, 18, 10, 225, 159, 7, 8, 9, 18, 10, 226, 160, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 161, 7, 8, 9, 18, 10, 225, 162, 7, 8, 9, 18, 10, 226, 163, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 164, 7, 8, 9, 18, 10, 225, 165, 7, 8, 9, 18, 10, 226, 166, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 167, 7, 8, 9, 18, 10, 225, 168, 7, 8, 9, 18, 10, 226, 169, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 170, 7, 8, 9, 18, 10, 225, 171, 7, 8, 9, 18, 10, 226, 172, 7, 8, 9, 18, 10, 234, 235, 236, 173, 7, 8, 9, 18, 10, 225, 174, 7, 8, 9, 18, 10, 226, 175, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 176, 7, 8, 9, 18, 10, 225, 177, 7, 8, 9, 18, 10, 226, 178, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 179, 7, 8, 9, 18, 10, 225, 180, 7, 8, 9, 18, 10, 226, 181, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 182, 7, 8, 9, 18, 10, 225, 183, 7, 8, 9, 18, 10, 226, 184, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 185, 7, 8, 9, 18, 10, 225, 186, 7, 8, 9, 18, 10, 226, 187, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 188, 7, 8, 9, 18, 10, 225, 189, 7, 8, 9, 18, 10, 226, 190, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 191, 7, 8, 9, 18, 10, 225, 192, 7, 8, 9, 18, 10, 226, 193, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 194, 7, 8, 9, 18, 10, 225, 195, 7, 8, 9, 18, 10, 226, 196, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 197, 7, 8, 9, 18, 10, 225, 198, 7, 8, 9, 18, 10, 226, 199, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 200, 7, 8, 9, 18, 10, 225, 201, 7, 8, 9, 18, 10, 226, 202, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 203, 7, 8, 9, 18, 10, 225, 204, 7, 8, 9, 18, 10, 226, 205, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 206, 7, 8, 9, 18, 10, 225, 207, 7, 8, 9, 18, 10, 226, 208, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 209, 7, 8, 9, 18, 10, 225, 210, 7, 8, 9, 18, 10, 226, 211, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 212, 7, 8, 9, 18, 10, 225, 213, 7, 8, 9, 18, 10, 226, 214, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 215, 7, 8, 9, 18, 10, 225, 216, 7, 8, 9, 18, 10, 226, 217, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 218, 7, 8, 9, 18, 10, 225, 219, 7, 8, 9, 18, 10, 226, 220, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 221, 7, 8, 9, 18, 10, 225, 222, 7, 8, 9, 18, 10, 226, 223, 7, 8, 9, 18, 10, 16, 255"
  );
}

async function verify_part_2(
  tmp_account_final_exp,
  withdraw_from_pubkey,
  withdraw_to_pubkey
) {
  // Creating Transaction Instructions

  const privateKeyDecoded = PRIVATE_KEY.split(",").map((s) => parseInt(s));
  const account = new solana.Account(privateKeyDecoded);
  var withdraw_to_pubkey = new solana.PublicKey(withdraw_to_pubkey);

  //creating an instruction for every entry in the instruction order array,
  //adding these instructions to a transaction and sending it
  for (let i = 0; i < instruction_order_verify_part_2.length; i++) {
    let tx = new solana.Transaction();

    //j is the number of instructions in one transactions
    for (
      let j = 0;
      j < 80 && j + i < instruction_order_verify_part_2.length;
      j++
    ) {
      //skipping the last instruction which crashes for visibility in terminal
      if (
        instruction_order_verify_part_2[j + i] == 103 ||
        instruction_order_verify_part_2[j + i] == 121 ||
        instruction_order_verify_part_2[j + i] == 122 ||
        instruction_order_verify_part_2[j + i] == 123 ||
        instruction_order_verify_part_2[j + i] == 124
      ) {
        continue;
      }

      var instruction_id = new Uint8Array(2);
      instruction_id[1] = 2;
      instruction_id[0] = instruction_order_verify_part_2[j + i];
      instruction_id = Buffer.from(instruction_id.buffer);
      tx.add(
        new solana.TransactionInstruction({
          programId: m_expo_program_id,
          keys: [
            { pubkey: account.publicKey, isSigner: true, isWritable: false },
            {
              pubkey: tmp_account_final_exp,
              isSigner: false,
              isWritable: true,
            },
          ],
          data: instruction_id,
        })
      );
    }
    //should be one less than intended since i already increases by 1
    i += 79;
    let x_intern = await tryTransaction(tx, account, connection);

    console.log(` ${i + 1} instr tx id ${x_intern}`);
  }

  // last transaction to view result of computation and withdraw (VERIFICATION AGAINST HC.VERIF KEY.f)
  let tx_last = new solana.Transaction();
  var instruction_id = new Uint8Array(2);
  instruction_id[0] = 121;
  instruction_id[1] = 2;
  instruction_id = Buffer.from(instruction_id.buffer);

  tx_last.add(
    new solana.TransactionInstruction({
      programId: m_expo_program_id,
      keys: [
        { pubkey: account.publicKey, isSigner: true, isWritable: false },
        { pubkey: tmp_account_final_exp, isSigner: false, isWritable: true }, // account_main
        {
          pubkey: merkle_tree_storage_acc_pkey,
          isSigner: false,
          isWritable: true,
        },
      ],
      data: instruction_id,
    })
  );
  instruction_id[0] = 122;
  instruction_id[1] = 2;
  instruction_id = Buffer.from(instruction_id.buffer);

  tx_last.add(
    new solana.TransactionInstruction({
      programId: m_expo_program_id,
      keys: [
        { pubkey: account.publicKey, isSigner: true, isWritable: false },
        { pubkey: tmp_account_final_exp, isSigner: false, isWritable: true }, // account_main
        {
          pubkey: merkle_tree_storage_acc_pkey,
          isSigner: false,
          isWritable: true,
        },
      ],
      data: instruction_id,
    })
  );
  instruction_id[0] = 123;
  instruction_id[1] = 2;
  instruction_id = Buffer.from(instruction_id.buffer);

  tx_last.add(
    new solana.TransactionInstruction({
      programId: m_expo_program_id,
      keys: [
        { pubkey: account.publicKey, isSigner: true, isWritable: false },
        { pubkey: tmp_account_final_exp, isSigner: false, isWritable: true }, // account_main
        {
          pubkey: merkle_tree_storage_acc_pkey,
          isSigner: false,
          isWritable: true,
        },
      ],
      data: instruction_id,
    })
  );
  instruction_id[0] = 124;
  instruction_id[1] = 2;
  instruction_id = Buffer.from(instruction_id.buffer);

  tx_last.add(
    new solana.TransactionInstruction({
      programId: m_expo_program_id,
      keys: [
        { pubkey: account.publicKey, isSigner: true, isWritable: false },
        { pubkey: tmp_account_final_exp, isSigner: false, isWritable: true }, // account_main
        {
          pubkey: merkle_tree_storage_acc_pkey,
          isSigner: false,
          isWritable: true,
        },
      ],
      data: instruction_id,
    })
  );
  instruction_id = new Uint8Array(2);
  instruction_id[0] = 103;
  instruction_id[1] = 2;
  instruction_id = Buffer.from(instruction_id.buffer);

  tx_last.add(
    new solana.TransactionInstruction({
      programId: m_expo_program_id,
      keys: [
        { pubkey: account.publicKey, isSigner: true, isWritable: false },
        { pubkey: tmp_account_final_exp, isSigner: false, isWritable: true }, // account_main
        { pubkey: withdraw_from_pubkey, isSigner: false, isWritable: true },
        { pubkey: withdraw_to_pubkey, isSigner: false, isWritable: true },
      ],
      data: instruction_id,
    })
  );
  let res = await tryTransaction(tx_last, account, connection);

  console.log("VP2: Call tx id ", res);
}

async function readAcc(account_pkey) {
  const accountInfo = await connection.getAccountInfo(
    new solana.PublicKey(account_pkey)
  );
  const data = Buffer.from(accountInfo.data);
  //console.dir(data, { depth: null });
  let v = "";
  let counter = 0;
  for (const item of data) {
    if (counter == 3841) {
      break;
    }
    v += item + " ";
    counter += 1;
  }
  console.log(v);
  //console.log(data);
}

async function getLeaves(pubkey) {
  console.log("get leaves from " + pubkey);
  const accountInfo = await connection.getAccountInfo(
    new solana.PublicKey(pubkey)
  );
  const data = [...accountInfo.data]; // read from buffer
  let min_range = 3937; // hc based on merkle_tree struct fields in program/lib.rs
  let max_range = 69473; // hc no. of leaves
  //check that tree height is compatible
  if (rust.bytes_to_int(data.slice(1, 9)) != 11) {
    throw "Tree height is not 11";
  }
  let next_index_bigInt = rust.bytes_to_int(data.slice(721, 729)); // hc, usize
  let next_index = Number(next_index_bigInt);
  console.log("nextIndex aftr rust: ", next_index);

  let leaves_bytes_range = data.slice(min_range, min_range + next_index * 32); // get only filled leaves. nextIndex seems to be "currentLeaves"

  return new Uint8Array(leaves_bytes_range); // unchunked
}

function createDeposit({ nullifier, secret }) {
  const deposit = { nullifier, secret }; // , preimage, commitment, commitmentHex, nullifierHash, nullifierHex
  deposit.preimage = Buffer.concat([deposit.nullifier, deposit.secret]);
  deposit.commitment = rust.hash_slice_u8(deposit.preimage); // DEPOSIT: INSERT INTO MERKLE TREE EXPECTS HEX string but found: arr (changed currently)
  deposit.commitmentHex = toHex(Buffer.from(deposit.commitment));
  deposit.nullifierHash = rust.hash_slice_u8(deposit.nullifier);
  deposit.nullifierHex = toHex(Buffer.from(deposit.nullifierHash));
  return deposit;
}

function parseNote(noteString) {
  // withdrawal takes note (unhashed hex)
  const noteRegex =
    /light-(?<currency>\w+)-(?<amount>[\d.]+)-0x(?<note>[0-9a-fA-F]{128})/g;
  const match = noteRegex.exec(noteString);
  if (!match) {
    throw new Error("The note has invalid format");
  }

  const buf = Buffer.from(match.groups.note, "hex");
  const nullifier = buf.slice(0, 32);
  const secret = buf.slice(32, 64);
  const deposit = createDeposit({ nullifier, secret });

  return {
    currency: match.groups.currency,
    amount: match.groups.amount,
    deposit,
  };
}
async function readAcc_Miller(account_pkey) {
  const accountInfo = await connection.getAccountInfo(
    new solana.PublicKey(account_pkey)
  );
  const data = Buffer.from(accountInfo.data);
  let init_data = data.slice(0, 220);
  let tx_integrity_hash = data.slice(190, 212);
  let f_r = data.slice(220, 576 + 220);
  let coeff2_r = data.slice(1728 + 220, 1824 + 220);
  let coeff1_r = data.slice(1824 + 220, 1920 + 220);
  let coeff0_r = data.slice(2208 + 220, 2304 + 220);
  let r_r = data.slice(3360 + 220, 3648 + 220);
  let proof_b_r = data.slice(3648 + 220, 3840 + 220);
  let prepared_inputs = data.slice(2908, 3004);

  let v = "";
  // let counter = 0;
  for (const item of init_data) {
    v += item + " ";
    // counter += 1;
  }
  console.log("init_data: ", v);
  v = "";
  // let counter = 0;
  for (const item of prepared_inputs) {
    v += item + " ";
    // counter += 1;
  }
  console.log("prepared_inputs: ", v);

  v = "";
  // let counter = 0;
  for (const item of f_r) {
    v += item + " ";
    // counter += 1;
  }
  console.log("f_r: ", v);

  v = "";
  // let counter = 0;
  for (const item of tx_integrity_hash) {
    v += item + " ";
    // counter += 1;
  }
  console.log("------------------------------------------");

  console.log("tx_integrity_hash: ", v);

  v = "";
  // let counter = 0;
  for (const item of coeff2_r) {
    v += item + " ";
    // counter += 1;
  }
  console.log("coeff2: ", v);

  v = "";
  // let counter = 0;
  for (const item of coeff1_r) {
    v += item + " ";
    // counter += 1;
  }
  console.log("coeff1: ", v);

  v = "";
  // let counter = 0;
  for (const item of coeff0_r) {
    v += item + " ";
    // counter += 1;
  }
  console.log("coeff0: ", v);

  v = "";
  // let counter = 0;
  for (const item of r_r) {
    v += item + " ";
    // counter += 1;
  }
  console.log("r: ", v);

  v = "";
  // let counter = 0;
  for (const item of proof_b_r) {
    v += item + " ";
    // counter += 1;
  }
  console.log("proofb: ", v);
}

async function check_deposit(noteString) {
  let parsed_note = parseNote(noteString);
  //checks
  let leaves = await getLeaves(merkle_tree_storage_acc_pkey);
  //leaf has been inserted
  let chunkSize = 32; // s: size of chunks
  let found_leaf = false;
  let leaf_position;
  for (let i = 0; i < leaves.length; i += chunkSize) {
    const chunk = leaves.slice(i, i + chunkSize);
    //console.log(`${parsed_note.deposit.commitment.slice(0,32)} == ${chunk}`)
    //console.log(parsed_note.deposit.commitment.slice(0,32) == chunk)
    if (parsed_note.deposit.commitment.toString() == chunk.toString()) {
      found_leaf = true;
      //console.log("found_leaf at position " + i /32)
      leaf_position = i / 32;
    }
  }

  console.assert(found_leaf == true, "Leaf not inserted");

  //nullifier exists
  const accountInfo = await connection.getAccountInfo(
    new solana.PublicKey(merkle_tree_storage_acc_pkey)
  );
  // const data = accountInfo.data
  const data = [...accountInfo.data]; // read from buffer
  // let leaves = []
  let min_range = 69481; // hc based on merkle_tree struct fields in program/lib.rs
  //let max_range = 69473 // hc no. of leaves
  let nullifiers = data.slice(min_range, data.length);

  let number_of_nullifiers = 0;
  let found_nullifier = false;
  for (let i = 0; i < nullifiers.length; i += chunkSize) {
    const chunk = nullifiers.slice(i, i + chunkSize);

    //console.log(`${parsed_note.deposit.nullifierHash} == ${chunk}`)
    if (parsed_note.deposit.nullifierHash.toString() == chunk.toString()) {
      found_nullifier = true;
      //console.log(found_nullifier)
    } else if (new Uint8Array(32).fill(0).toString() == chunk.toString()) {
      //break if reached end of submitted nullifiers
      break;
    }
    number_of_nullifiers += 1;
  }

  //console.assert(found_nullifier == false, "Nullifier found")
  let is_last_leaf = false;
  if (
    leaves.slice(leaves.length - 32, leaves.length).toString() ==
    parsed_note.deposit.commitment.toString()
  ) {
    is_last_leaf = true;
  }

  return {
    found_leaf: found_leaf,
    found_nullifier: found_nullifier,
    is_last_leaf: is_last_leaf,
    leaf_position: leaf_position,
    number_of_nullifiers: number_of_nullifiers,
  };
}

async function main() {
  // if in cli
  program.option(
    "-r, --rpc <URL>",
    "The RPC, CLI should interact with",
    "http://localhost:8899"
  );
  program
    .command("create_new_merkle_tree_acc")
    .description("init before first deposit")
    .action(async () =>
      getNodeConnection(solanaRPC).then(async function () {
        const privateKeyDecoded = PRIVATE_KEY.split(",").map((s) =>
          parseInt(s)
        );
        const account = new solana.Account(privateKeyDecoded);
        console.log(account.publicKey.toBase58());
        let merkle_tree_storage_acc;
        try {
          //initing merkletree account
          // for tree of heigh 18 8393001 bytes
          // for tree of heigh 11 135017 bytes
          var seed1 = crypto.randomBytes(15).toString("hex");
          console.log("seed New MerkleTree account: ", seed1);

          //hardcoded seed for easier testing
          merkle_tree_storage_acc = await create_program_acc(
            account,
            seed1,
            135057,
            m_expo_program_id
          );
          console.log("New merkletree pubkey: ", merkle_tree_storage_acc);
          console.log(
            "New MerkleTree account: ",
            Uint8Array.from(
              new solana.PublicKey(merkle_tree_storage_acc).toBuffer()
            )
          );

          console.log(
            `storage account for merkle tree of height 2 created with address: ${hash_tmp_account} \n`
          );
        } catch {}
        //await readAcc(merkle_tree_storage_acc_pkey);

        // build merkletree:
        //let note = await build_merkle_tree();
        console.log(`\nmerkletree state after initialization:`);
        //await readAcc(merkle_tree_storage_acc); //:ok smth
      })
    );
  program
    .command("init_merkle_tree")
    .description("init before first deposit")
    .action(async () =>
      getNodeConnection(solanaRPC).then(async function () {
        const privateKeyDecoded = PRIVATE_KEY.split(",").map((s) =>
          parseInt(s)
        );
        const account = new solana.Account(privateKeyDecoded);
        console.log(account.publicKey.toBase58());
        let merkle_tree_storage_acc;
        try {
          //initing merkletree account
          // for tree of heigh 18 8393001 bytes
          // for tree of heigh 11 135017 bytes
          //b1c65b61682181d3db36d3dab40bb0

          //hardcoded seed for easier testing
          merkle_tree_storage_acc = await create_program_acc(
            account,
            "test1",
            135057,
            m_expo_program_id
          );

          console.log(
            `storage account for merkle tree of height 2 created with address: ${hash_tmp_account} \n`
          );
        } catch {}
        //await readAcc(merkle_tree_storage_acc_pkey);

        // build merkletree:
        let note = await build_merkle_tree();
        console.log(`\nmerkletree state after initialization:`);
        await readAcc(merkle_tree_storage_acc); //:ok smth
      })
    );

  program
    .command("deposit <currency> <amount>")
    .description(
      "Submit a deposit of specified currency (SOL) and amount from default SOL account and return the resulting note."
    )
    .action(async (currency, amount) =>
      getNodeConnection(solanaRPC).then(async function () {
        const privateKeyDecoded = PRIVATE_KEY.split(",").map((s) =>
          parseInt(s)
        );
        var account = new solana.Account(privateKeyDecoded);

        console.log(account.publicKey.toBase58());
        var seed1 = crypto.randomBytes(15).toString("hex");

        var deposit = createDeposit({
          nullifier: crypto.randomBytes(32),
          secret: crypto.randomBytes(32),
        });
        var note = toHex(deposit.preimage, 64);

        var hash_tmp_account;
        try {
          //initing random hash_storage_acc
          hash_tmp_account = await create_program_acc(
            account,
            seed1,
            217,
            m_expo_program_id
          );
          console.log(
            `temporary storage account for poseidon hash created with address: \n ${hash_tmp_account} \n`
          );
        } catch {}
        console.log(
          `\n ----------- starting merkletree insert of new leaf ----------- \n`
        );
        await insert_into_merkle_tree(
          hash_tmp_account,
          deposit.commitment,
          amount
        ); // deposit
        console.log(`\n ----------- leaf insert successful ----------- \n`);
        //console.log(`merkletree state after insert:`)
        console.log(
          " \n PLEASE SAVE YOUR NOTE: ",
          "light-" + currency + "-" + amount + "-" + note
        ); // note includes "0x". see toHex

        let note_for_check = "light-" + currency + "-" + amount + "-" + note;
        let check_values = await check_deposit(note_for_check);
        console.assert(
          check_values.found_leaf == true,
          "leaf was not inserted successfully"
        );
        /*
        if (check_values.found_leaf != true) {
          throw new Error("leaf was not inserted successfully");
        }*/
        console.assert(
          check_values.found_nullifier == false,
          "nullifier should not exist already"
        );
        /*
        if (check_values.found_nullifier == true) {
          throw new Error("leaf was not inserted successfully");
        }*/
        console.assert(
          check_values.is_last_leaf == true,
          "inserted leaf should be last leaf"
        );
        /*
        if (check_values.is_last_leaf != true) {
          throw new Error("inserted leaf should be last leaf");
        }*/
        console.log("leaf_position = ", check_values.leaf_position);
        console.log(
          "number of withdrawals: ",
          check_values.number_of_nullifiers
        );
        // deposit sol
        currency = currency.toLowerCase();
        // return note
        console.log(
          " \n PLEASE SAVE YOUR NOTE: ",
          "light-" + currency + "-" + amount + "-" + note
        ); // note includes "0x". see toHex
      })
    );

  program
    .command("withdraw <note> <recipient>")
    .description(
      "Withdraw a note to a recipient account using specified private key."
    )
    .action(async (noteString, recipient) =>
      getNodeConnection(solanaRPC).then(async function () {
        console.log(`starting withdrawal with note: ${noteString} \n`);
        const privateKeyDecoded = PRIVATE_KEY.split(",").map((s) =>
          parseInt(s)
        );
        const account = new solana.Account(privateKeyDecoded);
        //console.log(account.publicKey.toBase58());
        var seed1 = crypto.randomBytes(15).toString("hex");
        var seed2 = (seed1 = crypto.randomBytes(15).toString("hex"));
        var seed3 = (seed1 = crypto.randomBytes(15).toString("hex"));

        // create new random program accounts (verif1,verif2)
        let tmp_account_miller_loop = await create_program_acc(
          account,
          seed1,
          4972,
          m_expo_program_id
        ); // curr: 2881
        console.log(
          `temporary storage account for miller loop (verifier part 1) created with address: \n ${tmp_account_miller_loop} \n`
        );

        let tmp_account_final_exp = await create_program_acc(
          account,
          seed2,
          4972,
          m_expo_program_id
        );
        console.log(
          `temporary storage account for final exponentiation (verifier part 2) created with address: \n ${tmp_account_final_exp} \n`
        );

        let tmp_account_prepare_inputs = await create_program_acc(
          account,
          seed3,
          4972,
          prep_inputs_program_id
        ); // own program
        console.log(
          `temporary storage account for preparig inputs on chain created with address: \n ${tmp_account_prepare_inputs} \n`
        );

        // hash note from noteString into deposit obj and send to withdraw
        const { currency, amount, deposit } = parseNote(noteString);

        console.log(merkle_tree_storage_acc_pkey);
        let leaves = await getLeaves(merkle_tree_storage_acc_pkey); // return nested array, i: hc merkletree pool pubkey
        //console.log("DEPOPREIMG:  ", deposit.preimage);
        console.log("number of leaves in leave slice: ", leaves.length / 32);
        console.log(
          "------------------------- Creating Proof -------------------------"
        );
        // DYNAMIC INPUTS/LEAF
        let f_p_coeffs = rust.create_f_p_coeffs(
          [...deposit.preimage],
          leaves,
          Uint8Array.from(new solana.PublicKey(recipient).toBuffer()),
          Uint8Array.from(account.publicKey.toBuffer()),
          leaves.length
        ); // Buffer.from(leaves)? // unhashed commitment {nullifier,secret /64}// as slice _> returns vec

        let inputs_bytes = f_p_coeffs.slice(0, 208);
        let proof_a_bytes = f_p_coeffs.slice(208, 304); // 96
        let proof_b_bytes = f_p_coeffs.slice(304, 496); // 192
        let proof_c_bytes = f_p_coeffs.slice(496, 592); // 96

        console.log(`\n ----------- preparing public inputs ----------- \n`);
        await fill_p(tmp_account_prepare_inputs, inputs_bytes); // ==> this needs to call different  program id, + adapt write last instruction into main prgm v1

        console.log(
          `\n ----------- starting miller loop (verifier part 1) ----------- \n`
        );
        await verify_part_1(
          tmp_account_miller_loop,
          tmp_account_prepare_inputs,
          tmp_account_miller_loop,
          proof_a_bytes,
          proof_b_bytes,
          proof_c_bytes
        ); // replace with proof
        // await readAcc(tmp_account_final_exp);
        console.log(
          `\n ----------- starting final exponentiation (verifier part 2) ----------- \n`
        );
        await verify_part_2(
          tmp_account_miller_loop,
          merkle_tree_storage_acc_pkey,
          recipient
        );

        let check_values = await check_deposit(noteString);
        console.assert(
          check_values.found_leaf == true,
          "leaf was not inserted successfully"
        );
        if (check_values.found_leaf != true) {
          throw new Error("leaf was not inserted successfully");
        }
        console.assert(
          check_values.found_nullifier == true,
          "nullifier should exist"
        );

        if (check_values.found_nullifier != true) {
          throw new Error("nullifier should exist");
        }

        console.log("leaf_position = ", check_values.leaf_position);
        console.log(
          "number of withdrawals: ",
          check_values.number_of_nullifiers
        );

        console.log(`\n 1 Sol withdrawal to: ${recipient} successful`);
      })
    );

  program.command("test-prepare_inputs").action(async () =>
    getNodeConnection(solanaRPC).then(async function () {
      console.log("starting test...");
      const privateKeyDecoded = PRIVATE_KEY.split(",").map((s) => parseInt(s));
      const account = new solana.Account(privateKeyDecoded);

      // var acc_main = new solana.PublicKey(tmp_account_miller_loop);
      //console.log("Tx sent from Account: " , account.publicKey.toBase58());
      //console.log("Tx account main: ",acc_main.toBase58());

      let tx1 = new solana.Transaction();
      var instruction_id = new Uint8Array(2);
      instruction_id[0] = 0;
      instruction_id[1] = 9;
      instruction_id = Buffer.from(instruction_id.buffer);

      tx1.add(
        new solana.TransactionInstruction({
          programId: m_expo_program_id,
          keys: [
            { pubkey: account.publicKey, isSigner: true, isWritable: false },
            // { pubkey: acc_main, isSigner: false, isWritable: true},
          ],
          data: instruction_id,
        })
      );

      tx1.recentBlockhash = await connection.getRecentBlockhash();
      // console.log("#b tx id ", tx1);

      let x = await solana.sendAndConfirmTransaction(
        // breaking here
        connection,
        tx1,
        [account],
        {
          commitment: "singleGossip",
          preflightCommitment: "singleGossip",
        }
      );
      console.log("Call TEST tx id ", x);
    })
  );
  program
    .command("get")
    .description(
      "Show all merkletree roots saved in the account onchain for testing."
    )
    .action(async () =>
      getNodeConnection(solanaRPC).then(async function () {
        //await readAccMerkletreeRoots(merkle_tree_storage_acc_pkey); //:ok smth
        //await readAccMerkletreeNullifier(merkle_tree_storage_acc_pkey);
        await readAccMerkletreeRoots(
          merkle_tree_storage_acc_pkey,
          [
            237, 216, 33, 38, 249, 20, 29, 126, 72, 20, 93, 173, 185, 193, 33,
            182, 207, 164, 89, 154, 182, 194, 29, 218, 168, 147, 206, 243, 37,
            17, 197, 4,
          ]
        );
      })
    );
  program
    .command("test_conversion")
    .description(
      "Show all merkletree roots saved in the account onchain for testing."
    )
    .action(async () =>
      getNodeConnection(solanaRPC).then(async function () {
        //await readAccMerkletreeRoots(merkle_tree_storage_acc_pkey); //:ok smth
        let pubkey = new solana.PublicKey(merkle_tree_storage_acc_pkey);

        console.log(
          Uint8Array.from(
            new solana.PublicKey('55a3HihdEDCoC9LNuBCPaBuEMSn5wFX58LrYYRVbBxot').toBuffer()
          )
        );
        console.log(
          "[251, 30, 194, 174, 168, 85, 13, 188, 134, 0, 17, 157, 187, 32, 113, 104, 134, 138, 82, 128, 95, 206, 76, 34, 177, 163, 246, 27, 109, 207, 2, 85]"
        );
      })
    );

  program.parse(process.argv);
}

async function readAccMerkletreeRoots(account_pkey, root) {
  const accountInfo = await connection.getAccountInfo(
    new solana.PublicKey(account_pkey)
  );
  const data = [...accountInfo.data];

  //console.dir(data, { depth: null });
  let current_root_index = rust.bytes_to_int(data.slice(737 - 24, 737 - 16));
  console.log("curren root index", current_root_index.toString());

  if (root) {
    let chunkSize = 32; // s: size of chunks
    let found_root = false;
    let leaf_position;
    for (let i = 737; i < 3937; i += chunkSize) {
      const chunk = data.slice(i, i + chunkSize);
      if (root.toString() == chunk.toString()) {
        found_root = true;

        console.log("found_leaf at position " + i / 32);
        leaf_position = i / 32;
      }
      if (parseInt(i / 32).toString() == (91).toString()) {
        console.log("Current root:");
        console.log(`${root.toString()} == ${chunk.toString()}`);
      }
      if (parseInt(i / 32).toString() == current_root_index.toString()) {
        console.log("Current root:");
        console.log(`${root.toString()} == ${chunk.toString()}`);
      }
    }
  }

  let v = "";
  let counter = 0;
  for (const item of data) {
    if (counter > 737 && counter < 1000) {
      v += item + " ";
      counter += 1;
    } else if (counter > 1000) {
      break;
    }
    counter += 1;
  }
}

async function readAccMerkletreeNullifier(account_pkey) {
  const accountInfo = await connection.getAccountInfo(
    new solana.PublicKey(account_pkey)
  );
  const data = Buffer.from(accountInfo.data);
  //console.dir(data, { depth: null });

  let v = "";
  let counter = 0;
  for (const item of data) {
    if (counter > 69481 && counter < 69801) {
      v += item + " ";
      counter += 1;
    } else if (counter > 69801) {
      break;
    }
    counter += 1;
  }
  console.log(v);
}

main();

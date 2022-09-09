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
exports.parseInputsToBytesArray = void 0;
var ffjavascript = require('ffjavascript');
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;
const parseInputsToBytesArray = function (data) {
    return __awaiter(this, void 0, void 0, function* () {
        var mydata = JSON.parse(data.toString());
        for (var i in mydata) {
            mydata[i] = Array.from(leInt2Buff(unstringifyBigInts(mydata[i]), 32)).reverse();
        }
        // console.log(mydata)
        // let x = [];
        // mydata.map((array) => {
        //     array.map((byte) => {
        //         x.push(byte);
        //     });
        // });
        // console.log(x.toString())

        return mydata;
    });
};
exports.parseInputsToBytesArray = parseInputsToBytesArray;

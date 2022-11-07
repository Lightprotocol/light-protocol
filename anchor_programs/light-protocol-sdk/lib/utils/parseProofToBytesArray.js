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
exports.parseProofToBytesArray = void 0;
var ffjavascript = require('ffjavascript');
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;
const parseProofToBytesArray = function (data) {
    return __awaiter(this, void 0, void 0, function* () {
        var mydata = JSON.parse(data.toString());
        for (var i in mydata) {
            if (i == 'pi_a') {
                for (var j in mydata[i]) {
                    mydata[i][j] = Array.from(leInt2Buff(unstringifyBigInts(mydata[i][j]), 32)).reverse();
                }
            }
            else
            if (i == 'pi_b') {
                for (var j in mydata[i]) {
                    for (var z in mydata[i][j]) {
                        mydata[i][j][z] = Array.from(leInt2Buff(unstringifyBigInts(mydata[i][j][z]), 32))//.reverse();
                    }
                }
            }
            else if (i == 'pi_c') {
                for (var j in mydata[i]) {
                    mydata[i][j] = Array.from(leInt2Buff(unstringifyBigInts(mydata[i][j]), 32)).reverse();
                }
            }
        }
        let mydataStripped = [
            mydata.pi_a[0],
            mydata.pi_a[1],
            Array.from([].concat.apply([], mydata.pi_b[0])).reverse(),
            Array.from([].concat.apply([], mydata.pi_b[1])).reverse(),
            mydata.pi_c[0],
            mydata.pi_c[1],
        ];
        var merged = [].concat.apply([], mydataStripped);

        return merged;
    });
};
exports.parseProofToBytesArray = parseProofToBytesArray;

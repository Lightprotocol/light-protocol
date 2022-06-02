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
exports.proofTimeoutPromise = void 0;
const parseInputsToBytesArray_1 = require("./utils/parseInputsToBytesArray");
const parseProofToBytesArray_1 = require("./utils/parseProofToBytesArray");
const prove_1 = require("./utils/prove");
const groth16TimeoutPromise_1 = require("./utils/groth16TimeoutPromise");
const proofTimeoutPromise = (input, inputUtxos) => __awaiter(void 0, void 0, void 0, function* () {
    var proofJson;
    var publicInputsJson;
    console.log(`PROOF INPUT WHAT CAN BE WRONG HERE`, input);
    console.log(`artifacts path -> ./artifacts/circuits/transaction${inputUtxos.length}`);
    yield (0, groth16TimeoutPromise_1.timeoutPromise)(40, (0, prove_1.prove)(input, `./artifacts/circuits/transaction${inputUtxos.length}`))
        .then((r) => {
        proofJson = r.proofJson;
        publicInputsJson = r.publicInputsJson;
    })
        .catch((e) => {
        console.log(e);
        throw new Error(`Your proof generation took too long. Please refresh the page and try again.`);
    });
    return {
        data: {
            publicInputsBytes: yield (0, parseInputsToBytesArray_1.parseInputsToBytesArray)(publicInputsJson),
            proofBytes: yield (0, parseProofToBytesArray_1.parseProofToBytesArray)(proofJson),
        },
    };
});
exports.proofTimeoutPromise = proofTimeoutPromise;

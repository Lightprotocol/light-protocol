"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.addInputInstructions = void 0;
const constants_1 = require("../constants");
const solana = require('@solana/web3.js');
const programId = new solana.PublicKey(constants_1.PROGRAM_ID);
const token = require("@solana/spl-token");
const addInputInstructions = ({ tx, payload = null, keys, ixDataLength = null, randU8 = null, }) => {
    let ixData;
    if (payload === null) {
        ixData = new Uint8Array(ixDataLength);
        ixData[0] = randU8;
    }
    else {
        ixData = payload;
    }
    tx.add(new solana.TransactionInstruction({
        programId: programId,
        keys: [
            {
                pubkey: keys[0],
                isSigner: true,
                isWritable: false,
            },
            keys.slice(1).map((key) => {
                if (key == token.TOKEN_PROGRAM_ID ||
                    key == solana.SystemProgram.programId ||
                    key == solana.SYSVAR_RENT_PUBKEY) {
                    return {
                        pubkey: key,
                        isSigner: false,
                        isWritable: false,
                    };
                }
                else {
                    return {
                        pubkey: key,
                        isSigner: false,
                        isWritable: true,
                    };
                }
            }),
        ].flat(),
        data: Buffer.from(ixData),
    }));
};
exports.addInputInstructions = addInputInstructions;

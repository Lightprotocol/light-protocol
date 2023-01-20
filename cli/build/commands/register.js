"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var fs = require("fs");
var saveUser = function () {
    var signature = "23232323232323";
    fs.writeFile("signature.txt", signature, function (err) {
        if (err)
            throw err;
        console.log("signature cached");
    });
    var decryptedUtxos = [{ test: "testString" }, 232323, "string"];
    fs.writeFile("utxos.txt", JSON.stringify(decryptedUtxos), function (err) {
        if (err)
            throw err;
        console.log("decrypted utxos cached");
    });
};
var readUser = function () {
    // read secret
    var signature;
    var decryptedUtxos = [];
    try {
        fs.readFile("signature.txt", "utf8", function (err, data) {
            if (err)
                throw err;
            console.log(data);
            signature = data;
        });
    }
    catch (e) {
        console.log(e);
    }
    try {
        fs.readFile("utxos.txt", "utf8", function (err, data) {
            if (err)
                throw err;
            console.log(JSON.parse(data));
            decryptedUtxos = JSON.parse(data);
        });
    }
    catch (e) {
        console.log(e);
    }
    // read encrypted user utxos (indices good enough!)
};
readUser();
console.log("save ?");
saveUser();
//# sourceMappingURL=register.js.map
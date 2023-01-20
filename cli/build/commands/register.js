"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var fs = require("fs");
var saveUser = function () {
    fs.writeFile("secret.txt", "private key : 121323232322323232323\n    \npublic key: 2323", function (err) {
        if (err)
            throw err;
        console.log("new user created. Back up your secret.txt file!");
    });
};
var readUser = function () {
    fs.readFile("secret.txt", "utf8", function (err, data) {
        if (err)
            throw err;
        console.log(data);
    });
};
saveUser();
readUser();
//# sourceMappingURL=register.js.map
"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.handler = exports.builder = exports.desc = exports.command = void 0;
exports.command = "greet <name>";
exports.desc = "Greet <name> with Hello";
var builder = function (yargs) {
    return yargs
        .options({
        upper: { type: "boolean" },
    })
        .positional("name", { type: "string", demandOption: true });
};
exports.builder = builder;
var handler = function (argv) {
    var name = argv.name, upper = argv.upper;
    var greeting = "Hello ".concat(name);
    process.stdout.write(upper ? greeting.toUpperCase() : greeting);
    process.exit(0);
};
exports.handler = handler;
//# sourceMappingURL=mock.js.map
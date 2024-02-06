const { execSync } = require("child_process");
const path = require("path");

process.env.TS_NODE_PROJECT = path.resolve("tsconfig.json");
process.env.NODE_ENV = "development";

global.oclif = global.oclif || {};
global.oclif.columns = 80;

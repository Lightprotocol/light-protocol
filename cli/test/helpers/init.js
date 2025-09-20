import { execSync } from "child_process";
import path from "path";
import { fileURLToPath } from "url";
import { dirname } from "path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

process.env.TS_NODE_PROJECT = path.resolve(
  __dirname,
  "../../tsconfig.test.json",
);
process.env.NODE_ENV = "development";

global.oclif = global.oclif || {};
global.oclif.columns = 80;

// we cannot link to the cli because it produces a circular link with zk.js
import { createVerifyingkeyRsFileArgv } from "../../../cli/src/psp-utils/createRustVerifyingKey";

createVerifyingkeyRsFileArgv();

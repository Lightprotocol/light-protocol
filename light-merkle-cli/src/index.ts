#!/usr/bin/env node
import { program } from "commander";
import { getLightVersion } from "../utils/utils";
import { commands } from "./commands";


const version = getLightVersion();

commands.forEach((command) => {
  program.addCommand(command)

});

program.version(version);

program.parse();

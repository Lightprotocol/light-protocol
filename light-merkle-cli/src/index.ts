#!/usr/bin/env node
import { Option, program } from "commander";
import { getLightVersion } from "../utils/packageInfo";
import { commands } from "./commands/list";
import { authority } from "./commands/actions/authority";
import { initialize } from "./commands/actions";
import { verifier } from "./commands/actions/verifier";
import { pool } from "./commands/actions/pool";
import { configure } from "./commands/actions/configure";


const version = getLightVersion();

commands.forEach((el) => {
  program.command(el.name).description(el.description).action(el.action);
});

program.addCommand(authority)
program.addCommand(initialize)
program.addCommand(verifier)
program.addCommand(pool)
program.addCommand(configure)

program.version(version);

program.parse();

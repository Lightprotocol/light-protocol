#!/usr/bin/env node
import { Option, program } from "commander";
import { getLightVersion } from "../utils/packageInfo";
import { commands } from "./commands/list";
import { authority } from "./commands/actions/authority";


const version = getLightVersion();

commands.forEach((el) => {
  program.command(el.name).description(el.description).action(el.action);
});

program.addCommand(authority)

program.version(version);

program.parse();

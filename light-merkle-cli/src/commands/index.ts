import { authority } from "./authority";
import { configure } from "./configure";
import { initialize } from "./initialize";
import { pool } from "./pool";
import { print } from "./print";
import { verifier } from "./verifier";

export const commands = [initialize, authority, configure, pool, verifier, print]
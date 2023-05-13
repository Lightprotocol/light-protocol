oclif-hello-world
=================

oclif example Hello World CLI

[![oclif](https://img.shields.io/badge/cli-oclif-brightgreen.svg)](https://oclif.io)
[![Version](https://img.shields.io/npm/v/oclif-hello-world.svg)](https://npmjs.org/package/oclif-hello-world)
[![CircleCI](https://circleci.com/gh/oclif/hello-world/tree/main.svg?style=shield)](https://circleci.com/gh/oclif/hello-world/tree/main)
[![Downloads/week](https://img.shields.io/npm/dw/oclif-hello-world.svg)](https://npmjs.org/package/oclif-hello-world)
[![License](https://img.shields.io/npm/l/oclif-hello-world.svg)](https://github.com/oclif/hello-world/blob/main/package.json)

<!-- toc -->
* [Usage](#usage)
* [Commands](#commands)
<!-- tocstop -->
# Usage
<!-- usage -->
```sh-session
$ npm install -g light
$ light COMMAND
running command...
$ light (--version)
light/0.0.0 linux-x64 node-v16.17.0
$ light --help [COMMAND]
USAGE
  $ light COMMAND
...
```
<!-- usagestop -->
# Commands
<!-- commands -->
* [`light hello PERSON`](#light-hello-person)
* [`light hello world`](#light-hello-world)
* [`light help [COMMANDS]`](#light-help-commands)
* [`light plugins`](#light-plugins)
* [`light plugins:install PLUGIN...`](#light-pluginsinstall-plugin)
* [`light plugins:inspect PLUGIN...`](#light-pluginsinspect-plugin)
* [`light plugins:install PLUGIN...`](#light-pluginsinstall-plugin-1)
* [`light plugins:link PLUGIN`](#light-pluginslink-plugin)
* [`light plugins:uninstall PLUGIN...`](#light-pluginsuninstall-plugin)
* [`light plugins:uninstall PLUGIN...`](#light-pluginsuninstall-plugin-1)
* [`light plugins:uninstall PLUGIN...`](#light-pluginsuninstall-plugin-2)
* [`light plugins update`](#light-plugins-update)

## `light hello PERSON`

Say hello

```
USAGE
  $ light hello PERSON -f <value>

ARGUMENTS
  PERSON  Person to say hello to

FLAGS
  -f, --from=<value>  (required) Who is saying hello

DESCRIPTION
  Say hello

EXAMPLES
  $ oex hello friend --from oclif
  hello friend from oclif! (./src/commands/hello/index.ts)
```

_See code: [dist/commands/hello/index.ts](https://github.com/sleepyqadir/light/blob/v0.0.0/dist/commands/hello/index.ts)_

## `light hello world`

Say hello world

```
USAGE
  $ light hello world

DESCRIPTION
  Say hello world

EXAMPLES
  $ light hello world
  hello world! (./src/commands/hello/world.ts)
```

## `light help [COMMANDS]`

Display help for light.

```
USAGE
  $ light help [COMMANDS] [-n]

ARGUMENTS
  COMMANDS  Command to show help for.

FLAGS
  -n, --nested-commands  Include all nested commands in the output.

DESCRIPTION
  Display help for light.
```

_See code: [@oclif/plugin-help](https://github.com/oclif/plugin-help/blob/v5.2.9/src/commands/help.ts)_

## `light plugins`

List installed plugins.

```
USAGE
  $ light plugins [--core]

FLAGS
  --core  Show core plugins.

DESCRIPTION
  List installed plugins.

EXAMPLES
  $ light plugins
```

_See code: [@oclif/plugin-plugins](https://github.com/oclif/plugin-plugins/blob/v2.4.7/src/commands/plugins/index.ts)_

## `light plugins:install PLUGIN...`

Installs a plugin into the CLI.

```
USAGE
  $ light plugins:install PLUGIN...

ARGUMENTS
  PLUGIN  Plugin to install.

FLAGS
  -f, --force    Run yarn install with force flag.
  -h, --help     Show CLI help.
  -v, --verbose

DESCRIPTION
  Installs a plugin into the CLI.
  Can be installed from npm or a git url.

  Installation of a user-installed plugin will override a core plugin.

  e.g. If you have a core plugin that has a 'hello' command, installing a user-installed plugin with a 'hello' command
  will override the core plugin implementation. This is useful if a user needs to update core plugin functionality in
  the CLI without the need to patch and update the whole CLI.


ALIASES
  $ light plugins add

EXAMPLES
  $ light plugins:install myplugin 

  $ light plugins:install https://github.com/someuser/someplugin

  $ light plugins:install someuser/someplugin
```

## `light plugins:inspect PLUGIN...`

Displays installation properties of a plugin.

```
USAGE
  $ light plugins:inspect PLUGIN...

ARGUMENTS
  PLUGIN  [default: .] Plugin to inspect.

FLAGS
  -h, --help     Show CLI help.
  -v, --verbose

GLOBAL FLAGS
  --json  Format output as json.

DESCRIPTION
  Displays installation properties of a plugin.

EXAMPLES
  $ light plugins:inspect myplugin
```

## `light plugins:install PLUGIN...`

Installs a plugin into the CLI.

```
USAGE
  $ light plugins:install PLUGIN...

ARGUMENTS
  PLUGIN  Plugin to install.

FLAGS
  -f, --force    Run yarn install with force flag.
  -h, --help     Show CLI help.
  -v, --verbose

DESCRIPTION
  Installs a plugin into the CLI.
  Can be installed from npm or a git url.

  Installation of a user-installed plugin will override a core plugin.

  e.g. If you have a core plugin that has a 'hello' command, installing a user-installed plugin with a 'hello' command
  will override the core plugin implementation. This is useful if a user needs to update core plugin functionality in
  the CLI without the need to patch and update the whole CLI.


ALIASES
  $ light plugins add

EXAMPLES
  $ light plugins:install myplugin 

  $ light plugins:install https://github.com/someuser/someplugin

  $ light plugins:install someuser/someplugin
```

## `light plugins:link PLUGIN`

Links a plugin into the CLI for development.

```
USAGE
  $ light plugins:link PLUGIN

ARGUMENTS
  PATH  [default: .] path to plugin

FLAGS
  -h, --help     Show CLI help.
  -v, --verbose

DESCRIPTION
  Links a plugin into the CLI for development.
  Installation of a linked plugin will override a user-installed or core plugin.

  e.g. If you have a user-installed or core plugin that has a 'hello' command, installing a linked plugin with a 'hello'
  command will override the user-installed or core plugin implementation. This is useful for development work.


EXAMPLES
  $ light plugins:link myplugin
```

## `light plugins:uninstall PLUGIN...`

Removes a plugin from the CLI.

```
USAGE
  $ light plugins:uninstall PLUGIN...

ARGUMENTS
  PLUGIN  plugin to uninstall

FLAGS
  -h, --help     Show CLI help.
  -v, --verbose

DESCRIPTION
  Removes a plugin from the CLI.

ALIASES
  $ light plugins unlink
  $ light plugins remove
```

## `light plugins:uninstall PLUGIN...`

Removes a plugin from the CLI.

```
USAGE
  $ light plugins:uninstall PLUGIN...

ARGUMENTS
  PLUGIN  plugin to uninstall

FLAGS
  -h, --help     Show CLI help.
  -v, --verbose

DESCRIPTION
  Removes a plugin from the CLI.

ALIASES
  $ light plugins unlink
  $ light plugins remove
```

## `light plugins:uninstall PLUGIN...`

Removes a plugin from the CLI.

```
USAGE
  $ light plugins:uninstall PLUGIN...

ARGUMENTS
  PLUGIN  plugin to uninstall

FLAGS
  -h, --help     Show CLI help.
  -v, --verbose

DESCRIPTION
  Removes a plugin from the CLI.

ALIASES
  $ light plugins unlink
  $ light plugins remove
```

## `light plugins update`

Update installed plugins.

```
USAGE
  $ light plugins update [-h] [-v]

FLAGS
  -h, --help     Show CLI help.
  -v, --verbose

DESCRIPTION
  Update installed plugins.
```
<!-- commandsstop -->

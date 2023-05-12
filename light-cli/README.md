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
$ npm install -g light-cli
$ light-cli COMMAND
running command...
$ light-cli (--version)
light-cli/0.0.0 linux-x64 node-v16.17.0
$ light-cli --help [COMMAND]
USAGE
  $ light-cli COMMAND
...
```
<!-- usagestop -->
# Commands
<!-- commands -->
* [`light-cli hello PERSON`](#light-cli-hello-person)
* [`light-cli hello world`](#light-cli-hello-world)
* [`light-cli help [COMMANDS]`](#light-cli-help-commands)
* [`light-cli plugins`](#light-cli-plugins)
* [`light-cli plugins:install PLUGIN...`](#light-cli-pluginsinstall-plugin)
* [`light-cli plugins:inspect PLUGIN...`](#light-cli-pluginsinspect-plugin)
* [`light-cli plugins:install PLUGIN...`](#light-cli-pluginsinstall-plugin-1)
* [`light-cli plugins:link PLUGIN`](#light-cli-pluginslink-plugin)
* [`light-cli plugins:uninstall PLUGIN...`](#light-cli-pluginsuninstall-plugin)
* [`light-cli plugins:uninstall PLUGIN...`](#light-cli-pluginsuninstall-plugin-1)
* [`light-cli plugins:uninstall PLUGIN...`](#light-cli-pluginsuninstall-plugin-2)
* [`light-cli plugins update`](#light-cli-plugins-update)

## `light-cli hello PERSON`

Say hello

```
USAGE
  $ light-cli hello PERSON -f <value>

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

_See code: [dist/commands/hello/index.ts](https://github.com/sleepyqadir/light-cli/blob/v0.0.0/dist/commands/hello/index.ts)_

## `light-cli hello world`

Say hello world

```
USAGE
  $ light-cli hello world

DESCRIPTION
  Say hello world

EXAMPLES
  $ light-cli hello world
  hello world! (./src/commands/hello/world.ts)
```

## `light-cli help [COMMANDS]`

Display help for light-cli.

```
USAGE
  $ light-cli help [COMMANDS] [-n]

ARGUMENTS
  COMMANDS  Command to show help for.

FLAGS
  -n, --nested-commands  Include all nested commands in the output.

DESCRIPTION
  Display help for light-cli.
```

_See code: [@oclif/plugin-help](https://github.com/oclif/plugin-help/blob/v5.2.9/src/commands/help.ts)_

## `light-cli plugins`

List installed plugins.

```
USAGE
  $ light-cli plugins [--core]

FLAGS
  --core  Show core plugins.

DESCRIPTION
  List installed plugins.

EXAMPLES
  $ light-cli plugins
```

_See code: [@oclif/plugin-plugins](https://github.com/oclif/plugin-plugins/blob/v2.4.7/src/commands/plugins/index.ts)_

## `light-cli plugins:install PLUGIN...`

Installs a plugin into the CLI.

```
USAGE
  $ light-cli plugins:install PLUGIN...

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
  $ light-cli plugins add

EXAMPLES
  $ light-cli plugins:install myplugin 

  $ light-cli plugins:install https://github.com/someuser/someplugin

  $ light-cli plugins:install someuser/someplugin
```

## `light-cli plugins:inspect PLUGIN...`

Displays installation properties of a plugin.

```
USAGE
  $ light-cli plugins:inspect PLUGIN...

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
  $ light-cli plugins:inspect myplugin
```

## `light-cli plugins:install PLUGIN...`

Installs a plugin into the CLI.

```
USAGE
  $ light-cli plugins:install PLUGIN...

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
  $ light-cli plugins add

EXAMPLES
  $ light-cli plugins:install myplugin 

  $ light-cli plugins:install https://github.com/someuser/someplugin

  $ light-cli plugins:install someuser/someplugin
```

## `light-cli plugins:link PLUGIN`

Links a plugin into the CLI for development.

```
USAGE
  $ light-cli plugins:link PLUGIN

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
  $ light-cli plugins:link myplugin
```

## `light-cli plugins:uninstall PLUGIN...`

Removes a plugin from the CLI.

```
USAGE
  $ light-cli plugins:uninstall PLUGIN...

ARGUMENTS
  PLUGIN  plugin to uninstall

FLAGS
  -h, --help     Show CLI help.
  -v, --verbose

DESCRIPTION
  Removes a plugin from the CLI.

ALIASES
  $ light-cli plugins unlink
  $ light-cli plugins remove
```

## `light-cli plugins:uninstall PLUGIN...`

Removes a plugin from the CLI.

```
USAGE
  $ light-cli plugins:uninstall PLUGIN...

ARGUMENTS
  PLUGIN  plugin to uninstall

FLAGS
  -h, --help     Show CLI help.
  -v, --verbose

DESCRIPTION
  Removes a plugin from the CLI.

ALIASES
  $ light-cli plugins unlink
  $ light-cli plugins remove
```

## `light-cli plugins:uninstall PLUGIN...`

Removes a plugin from the CLI.

```
USAGE
  $ light-cli plugins:uninstall PLUGIN...

ARGUMENTS
  PLUGIN  plugin to uninstall

FLAGS
  -h, --help     Show CLI help.
  -v, --verbose

DESCRIPTION
  Removes a plugin from the CLI.

ALIASES
  $ light-cli plugins unlink
  $ light-cli plugins remove
```

## `light-cli plugins update`

Update installed plugins.

```
USAGE
  $ light-cli plugins update [-h] [-v]

FLAGS
  -h, --help     Show CLI help.
  -v, --verbose

DESCRIPTION
  Update installed plugins.
```
<!-- commandsstop -->

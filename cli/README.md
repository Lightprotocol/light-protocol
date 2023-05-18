## Light Protocol CLI

Helps you build Private Solana Programs.

Find the code here: https://github.com/Lightprotocol/light-protocol/tree/main/cli

### Installation

Install with npm or yarn:

```
npm install -g @lightprotocol/light
```

```
yarn global add @lightprotocol/light
```

### Commands
Sets up all the boilerplate you need, includes a PSP template:
```
light init --name <project-name>
```

Builds the project:
```
light build --name <project-name>
```

  optional flags:  ```--ptau <ptau-parameter> --circuitDir <directory-containing-a-.light-file>```


Runs tests:
```
light test --projectName <project-name> --programAddress <program-address>
```

<p align="center">
  <img src="https://github.com/ldiego08/light-protocol/raw/main/assets/logo.svg" width="90" />
</p>

<h1 align="center">@lightprotocol/stateless.js</h1>

<p align="center">
  <b>This is the JavaScript SDK for building Solana applications with ZK Compression for Node and web.</b>
</p>

<p align="center">
  <a href="https://badge.fury.io/js/@lightprotocol%2Fstateless.js">
    <img src="https://badge.fury.io/js/@lightprotocol%2Fstateless.js.svg" alt="package npm version" height="18" />
  </a>
  <img src="https://img.shields.io/npm/l/%40lightprotocol%2Fstateless.js" alt="package license" height="18">
  <img src="https://img.shields.io/npm/dw/%40lightprotocol%2Fstateless.js" alt="package weekly downloads" height="18" />
</p>

## Usage

### Installation

Install this package in your project by running the following terminal command:

```bin
npm install --save @lightprotocol/stateless.js
```

## Documentation and Examples

For a more detailed documentation on usage, please check [the respective section at the ZK Compression documentation.](https://www.zkcompression.com/developers/typescript-client)

For example implementations, including web and Node, refer to the respective repositories:

-   [Web application example implementation](https://github.com/Lightprotocol/example-web-client)

-   [Node server example implementation](https://github.com/Lightprotocol/example-nodejs-client)

## Troubleshooting

Have a question or a problem?
Feel free to ask in the [Light](https://discord.gg/CYvjBgzRFP) and [Helius](https://discord.gg/Uzzf6a7zKr) developer Discord servers. Please, include the following information:

-   A detailed description or context of the issue or what you are trying to achieve.
-   A code example that we can use to test and debug (if possible). Use [CodeSandbox](https://codesandbox.io/p/sandbox/vanilla-ts) or any other live environment provider.
-   A description or context of any errors you are encountering with stacktraces if available.

### Source Maps

We provide `index.js.map` for debugging. Exclude in production:

**Webpack:** Set `devtool` to `false`.
**Rollup:** Set `sourcemap` to `false`.
**TypeScript:** Set `sourceMap` to `false`.

## Contributing

Light and ZK Compression are open source protocols and very much welcome contributions. If you have a contribution, do not hesitate to send a PR to the respective repository or discuss in the linked developer Discord servers.

-   🐞 For bugs or feature requests, please open an
    [issue](https://github.com/lightprotocol/light-protocol/issues/new).
-   🔒 For security vulnerabilities, please follow the [security policy](https://github.com/Lightprotocol/light-protocol/blob/main/SECURITY.md).

## Additional Resources

-   [Light Protocol Repository](https://github.com/Lightprotocol/light-protocol)
-   [ZK Compression Official Documentation](https://www.zkcompression.com/)

## Disclaimer

All claims, content, designs, algorithms, estimates, roadmaps, specifications,
and performance measurements described in this project are done with Light
Protocol Labs' ("Labs") best efforts. It is up to the reader to check and
validate their accuracy and truthfulness. Furthermore nothing in this project
constitutes a solicitation for investment.

Any content produced by Labs or developer resources that Labs provides, are for
educational and inspiration purposes only. Labs does not encourage, induce or
sanction the deployment, integration or use of any such applications (including
the code comprising the Light blockchain protocol) in violation of applicable
laws or regulations and hereby prohibits any such deployment, integration or
use. This includes use of any such applications by the reader (a) in violation
of export control or sanctions laws of the United States or any other applicable
jurisdiction, (b) if the reader is located in or ordinarily resident in a
country or territory subject to comprehensive sanctions administered by the U.S.
Office of Foreign Assets Control (OFAC), or (c) if the reader is or is working
on behalf of a Specially Designated National (SDN) or a person subject to
similar blocking or denied party prohibitions.

The reader should be aware that U.S. export control and sanctions laws prohibit
U.S. persons (and other persons that are subject to such laws) from transacting
with persons in certain countries and territories or that are on the SDN list.
As a project based primarily on open-source software, it is possible that such
sanctioned persons may nevertheless bypass prohibitions, obtain the code
comprising the Light blockchain protocol (or other project code or applications)
and deploy, integrate, or otherwise use it. Accordingly, there is a risk to
individuals that other persons using the Light blockchain protocol may be
sanctioned persons and that transactions with such persons would be a violation
of U.S. export controls and sanctions law. This risk applies to individuals,
organizations, and other ecosystem participants that deploy, integrate, or use
the Light blockchain protocol code directly (e.g., as a node operator), and
individuals that transact on the Light blockchain protocol implementation
through clients, other kinds of nodes, third party interfaces, and/or wallet
software.

"use client";
import "@mantine/core/styles.css";
import "@mantine/notifications/styles.css";

import React, { ReactNode, useMemo } from "react";
import { MantineProvider, ColorSchemeScript } from "@mantine/core";
import { ModalsProvider } from "@mantine/modals";
import { Provider } from "jotai";
import { ConnectionProvider } from "@solana/wallet-adapter-react";
import { Notifications } from "@mantine/notifications";

import { theme } from "../styles/theme";

export default function RootLayout({ children }: { children: ReactNode }) {
  console.log("process.env.rpc", process.env.NEXT_PUBLIC_RPC!);
  const endpoint = useMemo(() => process.env.NEXT_PUBLIC_RPC!, []);

  return (
    <html lang="en">
      <head>
        <ColorSchemeScript />
        <link rel="shortcut icon" href="/favicon.svg" />
        <meta
          name="viewport"
          content="minimum-scale=1, initial-scale=1, width=device-width, user-scalable=no"
        />
      </head>
      <body>
        <Provider>
          <ConnectionProvider endpoint={endpoint}>
            <MantineProvider theme={theme}>
              <ModalsProvider>
                <Notifications />
                {children}
              </ModalsProvider>
            </MantineProvider>
          </ConnectionProvider>
        </Provider>
      </body>
    </html>
  );
}

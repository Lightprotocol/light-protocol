"use client";
// import dynamic from "next/dynamic";
import "@mantine/core/styles.css";
import React, { ReactNode, useMemo } from "react";
import { MantineProvider, ColorSchemeScript } from "@mantine/core";
import { ModalsProvider } from "@mantine/modals";
import { Provider } from "jotai";
import { ConnectionProvider } from "@solana/wallet-adapter-react";

import { theme } from "../styles/theme";

// This is a dynamic import that will only be loaded on the client-side
// const DynamicChildren = dynamic(
//   () =>
//     Promise.resolve(({ children }: { children: ReactNode }) => <>{children}</>),
//   { ssr: false }
// );

export default function RootLayout({ children }: { children: ReactNode }) {
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
                {/* <DynamicChildren>{children}</DynamicChildren> */}
                {children}
              </ModalsProvider>
            </MantineProvider>
          </ConnectionProvider>
        </Provider>
      </body>
    </html>
  );
}

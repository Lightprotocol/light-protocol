"use client";
import "../styles/globals.css";

import { Providers } from "../state/providers";

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className={"font: inter"}>
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}

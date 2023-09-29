"use client";
import "./globals.css";

import { Providers } from "./providers";

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

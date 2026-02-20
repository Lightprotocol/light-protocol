import { NextResponse } from "next/server";

export const dynamic = "force-dynamic";

const BACKEND_URL =
  process.env.FORESTER_API_URL || "http://127.0.0.1:8080";

const BACKEND_TIMEOUT_MS = Number(
  process.env.FORESTER_API_TIMEOUT_MS ?? 8000
);

function isAbortError(error: unknown): boolean {
  return (
    typeof error === "object" &&
    error !== null &&
    "name" in error &&
    (error as { name?: string }).name === "AbortError"
  );
}

function joinBackendUrl(path: string): string {
  const base = BACKEND_URL.replace(/\/+$/, "");
  return `${base}/${path}`;
}

export async function GET(
  _request: Request,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  const backendPath = path.join("/");
  const upstream = joinBackendUrl(backendPath);

  const timeoutMs =
    Number.isFinite(BACKEND_TIMEOUT_MS) && BACKEND_TIMEOUT_MS > 0
      ? BACKEND_TIMEOUT_MS
      : 8000;

  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);

  try {
    const response = await fetch(upstream, {
      cache: "no-store",
      signal: controller.signal,
    });

    const contentType = response.headers.get("content-type") ?? "";
    const payload = contentType.includes("application/json")
      ? await response.json()
      : { message: await response.text() };

    if (!response.ok) {
      return NextResponse.json(
        {
          error: `Forester backend returned ${response.status}`,
          upstream,
          details: payload,
        },
        { status: response.status }
      );
    }

    return NextResponse.json(payload, { status: response.status });
  } catch (error) {
    if (isAbortError(error)) {
      return NextResponse.json(
        {
          error: `Backend request timed out after ${timeoutMs}ms`,
          upstream,
        },
        { status: 504 }
      );
    }

    return NextResponse.json(
      { error: "Backend unavailable", upstream },
      { status: 502 }
    );
  } finally {
    clearTimeout(timer);
  }
}

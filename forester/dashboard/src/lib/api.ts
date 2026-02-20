const API_URL =
  process.env.NEXT_PUBLIC_FORESTER_API_URL ?? "/api";

const REQUEST_TIMEOUT_MS = Number(
  process.env.NEXT_PUBLIC_FORESTER_API_TIMEOUT_MS ?? 8000
);

export class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
    public serverMessage: string
  ) {
    super(message);
    this.name = "ApiError";
  }
}

function isAbortError(error: unknown): boolean {
  return (
    typeof error === "object" &&
    error !== null &&
    "name" in error &&
    (error as { name?: string }).name === "AbortError"
  );
}

export async function fetchApi<T>(path: string): Promise<T> {
  const url = `${API_URL}${path}`;
  const timeoutMs =
    Number.isFinite(REQUEST_TIMEOUT_MS) && REQUEST_TIMEOUT_MS > 0
      ? REQUEST_TIMEOUT_MS
      : 8000;

  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);

  let res: Response;
  try {
    res = await fetch(url, {
      cache: "no-store",
      signal: controller.signal,
    });
  } catch (e) {
    if (isAbortError(e)) {
      throw new Error(
        `Request to forester API timed out after ${timeoutMs}ms (${url}).`
      );
    }
    throw new Error(
      `Cannot connect to forester API at ${API_URL}. Make sure the API server is running.`
    );
  } finally {
    clearTimeout(timer);
  }

  if (!res.ok) {
    let serverMsg = "";
    try {
      const body = await res.json();
      serverMsg = body.error || JSON.stringify(body);
    } catch {
      serverMsg = await res.text().catch(() => "Unknown error");
    }
    throw new ApiError(
      `Forester API returned ${res.status}: ${serverMsg}`,
      res.status,
      serverMsg
    );
  }

  return res.json();
}

export const fetcher = <T>(path: string) => fetchApi<T>(path);

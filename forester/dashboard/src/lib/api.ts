const API_URL =
  process.env.NEXT_PUBLIC_FORESTER_API_URL || "http://localhost:8080";

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

export async function fetchApi<T>(path: string): Promise<T> {
  let res: Response;
  try {
    res = await fetch(`${API_URL}${path}`);
  } catch (e) {
    // Network error — server not reachable
    throw new Error(
      `Cannot connect to forester API at ${API_URL}. Make sure the API server is running.`
    );
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

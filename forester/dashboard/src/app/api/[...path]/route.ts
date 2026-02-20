import { NextResponse } from "next/server";

const BACKEND_URL =
  process.env.FORESTER_API_URL || "http://localhost:8080";

export async function GET(
  request: Request,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  const backendPath = path.join("/");
  try {
    const response = await fetch(`${BACKEND_URL}/${backendPath}`);
    const data = await response.json();
    return NextResponse.json(data, { status: response.status });
  } catch {
    return NextResponse.json(
      { error: "Backend unavailable" },
      { status: 502 }
    );
  }
}

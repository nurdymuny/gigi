export const maxDuration = 30;

export default async function handler(req, res) {
  const baseUrl = process.env.GIGI_URL || "https://gigi-stream.fly.dev";
  const apiKey = process.env.GIGI_API_KEY || "";

  // Reconstruct the path from the catch-all segments
  const segments = req.query.path;
  const path = Array.isArray(segments) ? segments.join("/") : (segments || "");
  const url = `${baseUrl}/${path}`;

  const headers = { "Content-Type": "application/json" };
  if (apiKey) {
    headers["X-API-Key"] = apiKey;
  }

  const fetchOpts = { method: req.method, headers };
  if (req.method !== "GET" && req.method !== "HEAD" && req.body) {
    fetchOpts.body = typeof req.body === "string" ? req.body : JSON.stringify(req.body);
  }

  try {
    const upstream = await fetch(url, fetchOpts);
    const contentType = upstream.headers.get("content-type") || "";

    // Forward status
    res.status(upstream.status);

    if (contentType.includes("application/json")) {
      const data = await upstream.json();
      res.json(data);
    } else {
      const text = await upstream.text();
      res.setHeader("Content-Type", contentType || "text/plain");
      res.send(text);
    }
  } catch (err) {
    res.status(502).json({ error: "Upstream unavailable", detail: err.message });
  }
}

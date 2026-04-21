export function resolveTtsText(token) {
  if (token && typeof token.tts_text === "string" && token.tts_text.length > 0) {
    return token.tts_text;
  }
  if (token && typeof token.reading === "string" && token.reading.length > 0) {
    return token.reading;
  }
  if (token && typeof token.surface === "string") {
    return token.surface;
  }
  return "";
}

function delay(ms) {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

export async function waitForBackendReady({ fetcher = fetch, retries = 6, delayMs = 500 } = {}) {
  let lastStatus = { status: "starting" };
  for (let attempt = 0; attempt < retries; attempt += 1) {
    try {
      const response = await fetcher("/api/health");
      if (response.ok) {
        lastStatus = await response.json();
        if (lastStatus.status === "ready") {
          return lastStatus;
        }
      }
    } catch (_error) {
      // Keep retrying until attempts are exhausted.
    }

    if (attempt < retries - 1) {
      await delay(delayMs * (attempt + 1));
    }
  }

  return lastStatus;
}

export async function analyzeTextRequest(text, fetcher = fetch) {
  const response = await fetcher("/api/analyze", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ text }),
  });
  if (!response.ok) {
    throw new Error(`analyze failed: ${response.status}`);
  }
  return response.json();
}

export async function lookupDictionaryRequest(term, fetcher = fetch) {
  const response = await fetcher(`/api/dictionary?term=${encodeURIComponent(term)}`);
  if (response.status === 404) {
    return null;
  }
  if (!response.ok) {
    throw new Error(`dictionary failed: ${response.status}`);
  }
  return response.json();
}

if (typeof window !== "undefined") {
  window.FudokiBackendApi = {
    resolveTtsText,
    waitForBackendReady,
    analyzeTextRequest,
    lookupDictionaryRequest,
  };
}

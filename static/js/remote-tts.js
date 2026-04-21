const STATES = Object.freeze({
  idle: "idle",
  loading: "loading",
  playing: "playing",
  error: "error",
});

function defaultBlobFactory(parts, options) {
  const BlobCtor = globalThis.Blob;
  if (typeof BlobCtor === "function") {
    return new BlobCtor(parts, options);
  }

  // Node tests inject URL APIs that ignore the blob argument, so a lightweight fallback is fine.
  return { parts, type: options?.type ?? "" };
}

function defaultUrlApi() {
  return {
    createObjectURL: (blob) => URL.createObjectURL(blob),
    revokeObjectURL: (value) => URL.revokeObjectURL(value),
  };
}

export function createRemoteTtsPlayer({
  audioFactory = () => new Audio(),
  blobFactory = defaultBlobFactory,
  urlApi = defaultUrlApi(),
  onStateChange = () => {},
} = {}) {
  let state = STATES.idle;
  let audio = null;
  let objectUrl = null;

  function emit(next) {
    state = next;
    try {
      onStateChange(next);
    } catch (_error) {
      // State listeners must not be able to break playback or cleanup.
    }
  }

  function revokeObjectUrl() {
    if (!objectUrl) return;
    try {
      urlApi.revokeObjectURL(objectUrl);
    } catch (_error) {
      // Ignore revoke errors; object URL lifecycle should be best-effort.
    } finally {
      objectUrl = null;
    }
  }

  function cleanupAudio() {
    if (!audio) return;

    try {
      audio.pause?.();
    } catch (_error) {}

    try {
      audio.src = "";
    } catch (_error) {}

    try {
      audio.currentTime = 0;
    } catch (_error) {}

    audio.onended = null;
    audio.onerror = null;
    audio = null;
  }

  function stopInternal({ emitIdle } = { emitIdle: true }) {
    cleanupAudio();
    revokeObjectUrl();
    if (emitIdle) emit(STATES.idle);
  }

  async function playResponse(response) {
    // Stop any current playback without emitting an intermediate "idle" state.
    stopInternal({ emitIdle: false });

    emit(STATES.loading);

    let buffer;
    try {
      buffer = await response.arrayBuffer();
    } catch (_error) {
      emit(STATES.error);
      return;
    }

    const contentType =
      response && response.headers && typeof response.headers.get === "function"
        ? response.headers.get("content-type") || response.headers.get("Content-Type") || ""
        : "";

    let blob;
    try {
      blob = blobFactory([buffer], { type: contentType || "" });
    } catch (_error) {
      emit(STATES.error);
      return;
    }

    try {
      objectUrl = urlApi.createObjectURL(blob);
    } catch (_error) {
      emit(STATES.error);
      return;
    }

    audio = audioFactory();
    audio.onended = () => {
      stopInternal({ emitIdle: true });
    };
    audio.onerror = () => {
      cleanupAudio();
      revokeObjectUrl();
      emit(STATES.error);
    };

    audio.src = objectUrl;

    try {
      await audio.play();
      emit(STATES.playing);
    } catch (_error) {
      cleanupAudio();
      revokeObjectUrl();
      emit(STATES.error);
    }
  }

  function stop() {
    stopInternal({ emitIdle: true });
  }

  function getState() {
    return state;
  }

  return Object.freeze({
    playResponse,
    stop,
    getState,
  });
}

if (typeof window !== "undefined") {
  window.FudokiRemoteTts = {
    createRemoteTtsPlayer,
  };
}


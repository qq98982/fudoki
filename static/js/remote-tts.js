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
  let generation = 0;
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

  function revokeObjectUrl(url) {
    if (!url) return;
    try {
      urlApi.revokeObjectURL(url);
    } catch (_error) {
      // Ignore revoke errors; object URL lifecycle should be best-effort.
    }
  }

  function cleanupAudio(audioToCleanup) {
    if (!audioToCleanup) return;

    try {
      audioToCleanup.pause?.();
    } catch (_error) {}

    try {
      audioToCleanup.src = "";
    } catch (_error) {}

    try {
      audioToCleanup.currentTime = 0;
    } catch (_error) {}

    audioToCleanup.onended = null;
    audioToCleanup.onerror = null;
  }

  function stopInternal({ emitIdle } = { emitIdle: true }) {
    const audioToStop = audio;
    const urlToRevoke = objectUrl;

    audio = null;
    objectUrl = null;

    cleanupAudio(audioToStop);
    revokeObjectUrl(urlToRevoke);
    if (emitIdle) {
      emit(STATES.idle);
    }
  }

  async function playResponse(response) {
    const playGeneration = (generation += 1);

    // Stop any current playback without emitting an intermediate "idle" state.
    stopInternal({ emitIdle: false });

    emit(STATES.loading);

    let buffer;
    try {
      buffer = await response.arrayBuffer();
    } catch (_error) {
      if (playGeneration === generation) emit(STATES.error);
      return;
    }

    if (playGeneration !== generation) return;

    const contentType =
      response && response.headers && typeof response.headers.get === "function"
        ? response.headers.get("content-type") || response.headers.get("Content-Type") || ""
        : "";

    let blob;
    try {
      blob = blobFactory([buffer], { type: contentType || "" });
    } catch (_error) {
      if (playGeneration === generation) emit(STATES.error);
      return;
    }

    if (playGeneration !== generation) return;

    let nextUrl = null;
    try {
      nextUrl = urlApi.createObjectURL(blob);
    } catch (_error) {
      if (playGeneration === generation) emit(STATES.error);
      return;
    }

    if (playGeneration !== generation) {
      revokeObjectUrl(nextUrl);
      return;
    }

    const nextAudio = audioFactory();
    const handlerGeneration = playGeneration;
    nextAudio.onended = () => {
      if (handlerGeneration !== generation) return;
      stopInternal({ emitIdle: true });
    };
    nextAudio.onerror = () => {
      if (handlerGeneration !== generation) return;
      stopInternal({ emitIdle: false });
      emit(STATES.error);
    };

    nextAudio.src = nextUrl;

    // Publish "current" only after we have a fully wired audio element + URL, guarded by generation.
    audio = nextAudio;
    objectUrl = nextUrl;

    try {
      await nextAudio.play();
      if (playGeneration === generation) emit(STATES.playing);
    } catch (_error) {
      if (playGeneration !== generation) {
        cleanupAudio(nextAudio);
        revokeObjectUrl(nextUrl);
        return;
      }

      stopInternal({ emitIdle: false });
      emit(STATES.error);
    }
  }

  function stop() {
    generation += 1;
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

import test from "node:test";
import assert from "node:assert/strict";

import { createRemoteTtsPlayer } from "../../static/js/remote-tts.js";

function createFakeAudio(log) {
  return {
    src: "",
    paused: true,
    currentTime: 0,
    onended: null,
    onerror: null,
    async play() {
      log.push(["play", this.src]);
      this.paused = false;
    },
    pause() {
      log.push(["pause", this.src]);
      this.paused = true;
    },
  };
}

test("remote player creates object urls and transitions into playing state", async () => {
  const log = [];
  const revoked = [];
  const states = [];
  const player = createRemoteTtsPlayer({
    audioFactory: () => createFakeAudio(log),
    urlApi: {
      createObjectURL: () => "blob:test-1",
      revokeObjectURL: (value) => revoked.push(value),
    },
    onStateChange: (state) => states.push(state),
  });

  await player.playResponse({
    headers: { get: () => "audio/mpeg" },
    arrayBuffer: async () => new Uint8Array([1, 2, 3]).buffer,
  });

  assert.deepEqual(states, ["loading", "playing"]);
  assert.deepEqual(log, [["play", "blob:test-1"]]);
  assert.deepEqual(revoked, []);
});

test("remote player stop revokes object urls and returns to idle", async () => {
  const revoked = [];
  const player = createRemoteTtsPlayer({
    audioFactory: () => createFakeAudio([]),
    urlApi: {
      createObjectURL: () => "blob:test-2",
      revokeObjectURL: (value) => revoked.push(value),
    },
    onStateChange: () => {},
  });

  await player.playResponse({
    headers: { get: () => "audio/mpeg" },
    arrayBuffer: async () => new Uint8Array([4, 5, 6]).buffer,
  });

  player.stop();

  assert.deepEqual(revoked, ["blob:test-2"]);
  assert.equal(player.getState(), "idle");
});

test("remote player ignores late onended from prior playback (does not stop newest)", async () => {
  const revoked = [];
  const log = [];
  const audios = [];
  let urlCounter = 0;
  const player = createRemoteTtsPlayer({
    audioFactory: () => {
      const audio = createFakeAudio(log);
      audios.push(audio);
      return audio;
    },
    urlApi: {
      createObjectURL: () => `blob:test-${(urlCounter += 1)}`,
      revokeObjectURL: (value) => revoked.push(value),
    },
    onStateChange: () => {},
  });

  await player.playResponse({
    headers: { get: () => "audio/mpeg" },
    arrayBuffer: async () => new Uint8Array([1]).buffer,
  });

  const lateOnEnded = audios[0].onended;

  await player.playResponse({
    headers: { get: () => "audio/mpeg" },
    arrayBuffer: async () => new Uint8Array([2]).buffer,
  });

  // Simulate a late event from the first audio instance after a new playback started.
  lateOnEnded?.();

  assert.deepEqual(revoked, ["blob:test-1"]);
  assert.equal(player.getState(), "playing");
});

test("remote player ignores stale async playResponse work when a newer call starts", async () => {
  const revoked = [];
  const log = [];
  let resolveFirst = null;
  const firstBufferPromise = new Promise((resolve) => {
    resolveFirst = resolve;
  });

  const player = createRemoteTtsPlayer({
    audioFactory: () => createFakeAudio(log),
    urlApi: {
      createObjectURL: () => "blob:test-latest",
      revokeObjectURL: (value) => revoked.push(value),
    },
    onStateChange: () => {},
  });

  const firstPlay = player.playResponse({
    headers: { get: () => "audio/mpeg" },
    arrayBuffer: async () => firstBufferPromise,
  });

  await player.playResponse({
    headers: { get: () => "audio/mpeg" },
    arrayBuffer: async () => new Uint8Array([9]).buffer,
  });

  resolveFirst(new Uint8Array([8]).buffer);
  await firstPlay;

  assert.deepEqual(log, [["play", "blob:test-latest"]]);
  assert.deepEqual(revoked, []);
  assert.equal(player.getState(), "playing");
});

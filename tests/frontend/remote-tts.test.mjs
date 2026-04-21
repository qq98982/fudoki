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


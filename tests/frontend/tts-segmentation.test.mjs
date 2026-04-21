import test from "node:test";
import assert from "node:assert/strict";
import vm from "node:vm";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const mainJsSource = readFileSync(resolve(__dirname, "../../static/main-js.js"), "utf8");
const ttsJsSource = readFileSync(resolve(__dirname, "../../static/js/tts.js"), "utf8");

function extractFunctionSource(source, functionName) {
  const marker = `function ${functionName}(`;
  const start = source.indexOf(marker);
  assert.notEqual(start, -1, `Could not find ${functionName} in source`);

  const bodyStart = source.indexOf("{", start);
  let depth = 0;
  let end = bodyStart;

  for (; end < source.length; end += 1) {
    const ch = source[end];
    if (ch === "{") depth += 1;
    if (ch === "}") {
      depth -= 1;
      if (depth === 0) {
        end += 1;
        break;
      }
    }
  }

  return source.slice(start, end);
}

function runSplit(text) {
  const splitFunction = extractFunctionSource(mainJsSource, "splitTextByPunctuation");
  const context = { input: text, result: null, String };
  vm.createContext(context);
  new vm.Script(`${splitFunction}\nresult = splitTextByPunctuation(input);`).runInContext(context);
  return context.result;
}

test("default content no longer includes the English tail sentence", () => {
  assert.doesNotMatch(mainJsSource, /Try Fudoki and enjoy Japanese language analysis!/);
});

test("title-like newline creates a stronger pause than ordinary line breaks", () => {
  const titleSegments = runSplit("外来語の話\nこれは本文の最初の文です");
  const bodySegments = runSplit("これは本文の途中にある少し長めの行であり\n次の行に続いています");
  const paragraphSegments = runSplit("見出し\n\nこれは次の段落です");

  assert.equal(titleSegments[0].text, "外来語の話");
  assert.equal(bodySegments[0].text, "これは本文の途中にある少し長めの行であり");
  assert.equal(paragraphSegments[0].text, "見出し");
  assert.ok(titleSegments[0].pause > bodySegments[0].pause);
  assert.ok(paragraphSegments[0].pause > titleSegments[0].pause);
});

test("speech preprocessing preserves line breaks before segmentation", () => {
  const mainSpeakSource = extractFunctionSource(mainJsSource, "speakWithPauses");
  const ttsSpeakSource = extractFunctionSource(ttsJsSource, "speakWithPauses");

  assert.doesNotMatch(mainSpeakSource, /\.replace\(\/\[\\s\\u00A0\]\+\/g, ' '\)/);
  assert.ok(mainSpeakSource.includes(".replace(/[^\\S\\n\\r\\u00A0]+/g, ' ')"));
  assert.doesNotMatch(ttsSpeakSource, /\.replace\(\/\[\\s\\u00A0\]\+\/g, ' '\)/);
  assert.ok(ttsSpeakSource.includes(".replace(/[^\\S\\n\\r\\u00A0]+/g, ' ')"));
});

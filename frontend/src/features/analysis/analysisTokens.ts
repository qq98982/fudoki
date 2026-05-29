import type { AnalyzeToken } from '../../types'

export type AnalysisDisplayItem =
  | { kind: 'token'; token: AnalyzeToken }
  | { kind: 'punct'; surface: string }

const kanaMap: Record<string, string> = {
  ア: 'a',
  イ: 'i',
  ウ: 'u',
  エ: 'e',
  オ: 'o',
  カ: 'ka',
  キ: 'ki',
  ク: 'ku',
  ケ: 'ke',
  コ: 'ko',
  サ: 'sa',
  シ: 'shi',
  ス: 'su',
  セ: 'se',
  ソ: 'so',
  タ: 'ta',
  チ: 'chi',
  ツ: 'tsu',
  テ: 'te',
  ト: 'to',
  ナ: 'na',
  ニ: 'ni',
  ヌ: 'nu',
  ネ: 'ne',
  ノ: 'no',
  ハ: 'ha',
  ヒ: 'hi',
  フ: 'fu',
  ヘ: 'he',
  ホ: 'ho',
  マ: 'ma',
  ミ: 'mi',
  ム: 'mu',
  メ: 'me',
  モ: 'mo',
  ヤ: 'ya',
  ユ: 'yu',
  ヨ: 'yo',
  ラ: 'ra',
  リ: 'ri',
  ル: 'ru',
  レ: 're',
  ロ: 'ro',
  ワ: 'wa',
  ヲ: 'o',
  ン: 'n',
  ガ: 'ga',
  ギ: 'gi',
  グ: 'gu',
  ゲ: 'ge',
  ゴ: 'go',
  ザ: 'za',
  ジ: 'ji',
  ズ: 'zu',
  ゼ: 'ze',
  ゾ: 'zo',
  ダ: 'da',
  ヂ: 'ji',
  ヅ: 'zu',
  デ: 'de',
  ド: 'do',
  バ: 'ba',
  ビ: 'bi',
  ブ: 'bu',
  ベ: 'be',
  ボ: 'bo',
  パ: 'pa',
  ピ: 'pi',
  プ: 'pu',
  ペ: 'pe',
  ポ: 'po',
  キャ: 'kya',
  キュ: 'kyu',
  キョ: 'kyo',
  シャ: 'sha',
  シュ: 'shu',
  ショ: 'sho',
  チャ: 'cha',
  チュ: 'chu',
  チョ: 'cho',
  ニャ: 'nya',
  ニュ: 'nyu',
  ニョ: 'nyo',
  ヒャ: 'hya',
  ヒュ: 'hyu',
  ヒョ: 'hyo',
  ミャ: 'mya',
  ミュ: 'myu',
  ミョ: 'myo',
  リャ: 'rya',
  リュ: 'ryu',
  リョ: 'ryo',
  ギャ: 'gya',
  ギュ: 'gyu',
  ギョ: 'gyo',
  ジャ: 'ja',
  ジュ: 'ju',
  ジョ: 'jo',
  ビャ: 'bya',
  ビュ: 'byu',
  ビョ: 'byo',
  ピャ: 'pya',
  ピュ: 'pyu',
  ピョ: 'pyo',
  // Hiragana (for robustness if any reading path produces hiragana)
  あ: 'a',
  い: 'i',
  う: 'u',
  え: 'e',
  お: 'o',
  か: 'ka',
  き: 'ki',
  く: 'ku',
  け: 'ke',
  こ: 'ko',
  さ: 'sa',
  し: 'shi',
  す: 'su',
  せ: 'se',
  そ: 'so',
  た: 'ta',
  ち: 'chi',
  つ: 'tsu',
  て: 'te',
  と: 'to',
  な: 'na',
  に: 'ni',
  ぬ: 'nu',
  ね: 'ne',
  の: 'no',
  は: 'ha',
  ひ: 'hi',
  ふ: 'fu',
  へ: 'he',
  ほ: 'ho',
  ま: 'ma',
  み: 'mi',
  む: 'mu',
  め: 'me',
  も: 'mo',
  や: 'ya',
  ゆ: 'yu',
  よ: 'yo',
  ら: 'ra',
  り: 'ri',
  る: 'ru',
  れ: 're',
  ろ: 'ro',
  わ: 'wa',
  を: 'o',
  ん: 'n',
  が: 'ga',
  ぎ: 'gi',
  ぐ: 'gu',
  げ: 'ge',
  ご: 'go',
  ざ: 'za',
  じ: 'ji',
  ず: 'zu',
  ぜ: 'ze',
  ぞ: 'zo',
  だ: 'da',
  ぢ: 'ji',
  づ: 'zu',
  で: 'de',
  ど: 'do',
  ば: 'ba',
  び: 'bi',
  ぶ: 'bu',
  べ: 'be',
  ぼ: 'bo',
  ぱ: 'pa',
  ぴ: 'pi',
  ぷ: 'pu',
  ぺ: 'pe',
  ぽ: 'po',
  きゃ: 'kya',
  きゅ: 'kyu',
  きょ: 'kyo',
  しゃ: 'sha',
  しゅ: 'shu',
  しょ: 'sho',
  ちゃ: 'cha',
  ちゅ: 'chu',
  ちょ: 'cho',
  にゃ: 'nya',
  にゅ: 'nyu',
  にょ: 'nyo',
  ひゃ: 'hya',
  ひゅ: 'hyu',
  ひょ: 'hyo',
  みゃ: 'mya',
  みゅ: 'myu',
  みょ: 'myo',
  りゃ: 'rya',
  りゅ: 'ryu',
  りょ: 'ryo',
  ぎゃ: 'gya',
  ぎゅ: 'gyu',
  ぎょ: 'gyo',
  じゃ: 'ja',
  じゅ: 'ju',
  じょ: 'jo',
  びゃ: 'bya',
  びゅ: 'byu',
  びょ: 'byo',
  ぴゃ: 'pya',
  ぴゅ: 'pyu',
  ぴょ: 'pyo',
}

export function tokenPosKey(token: AnalyzeToken) {
  const main = token.pos[0] ?? ''
  if (main.includes('名')) return 'noun'
  if (main.includes('動')) return 'verb'
  if (main.includes('形')) return 'adjective'
  if (main.includes('副')) return 'adverb'
  if (main.includes('助')) return 'particle'
  if (main.includes('感')) return 'interjection'
  if (main.includes('記号') || main.includes('補助記号')) return 'symbol'
  return 'other'
}

export function tokenSpeechText(token: AnalyzeToken) {
  return token.tts_text || token.reading || token.surface
}

export function lineSpeechText(tokens: AnalyzeToken[]) {
  return toDisplayItems(tokens)
    .map((item) => (item.kind === 'token' ? tokenSpeechText(item.token) : item.surface))
    .join('')
}

export function tokenRomaji(reading: string) {
  if (!reading) return ''

  let index = 0
  let output = ''
  while (index < reading.length) {
    const pair = reading.slice(index, index + 2)
    if (kanaMap[pair]) {
      output += kanaMap[pair]
      index += 2
      continue
    }

    const single = reading[index]
    if (single === 'ー') {
      const last = output.at(-1)
      if (last) output += last
      index += 1
      continue
    }

    output += kanaMap[single] ?? single.toLowerCase()
    index += 1
  }

  return output
}

export function toDisplayItems(tokens: AnalyzeToken[]) {
  const items: AnalysisDisplayItem[] = []
  let parentheticalDepth = 0

  for (const token of tokens) {
    const surface = token.surface ?? ''

    if (isMarkdownMarker(surface) || isDecorativeSymbol(surface)) {
      continue
    }

    if (isBlankToken(token)) {
      continue
    }

    if (isOpeningParen(surface)) {
      parentheticalDepth += 1
      continue
    }

    if (isClosingParen(surface)) {
      if (parentheticalDepth > 0) {
        parentheticalDepth -= 1
      }
      continue
    }

    if (parentheticalDepth > 0) {
      continue
    }

    if (isPlainPunctuation(token)) {
      items.push({ kind: 'punct', surface })
      continue
    }

    items.push({ kind: 'token', token })
  }

  return items
}

function isMarkdownMarker(surface: string) {
  return /^[#*_`>~=\-|]+$/.test(surface)
}

function isDecorativeSymbol(surface: string) {
  return /^[•·]+$/.test(surface)
}

function isOpeningParen(surface: string) {
  return ['(', '（', '[', '［', '【', '〔', '〈', '《'].includes(surface)
}

function isClosingParen(surface: string) {
  return [')', '）', ']', '］', '】', '〕', '〉', '》'].includes(surface)
}

function isPlainPunctuation(token: AnalyzeToken) {
  const mainPos = token.pos[0] ?? ''
  if (!(mainPos.includes('記号') || mainPos.includes('補助記号'))) {
    return false
  }

  return /^[。、！？「」『』…・、，。,:：;；!！?？]$/.test(token.surface)
}

function isBlankToken(token: AnalyzeToken) {
  const mainPos = token.pos[0] ?? ''
  return mainPos.includes('空白') || token.surface.trim().length === 0
}

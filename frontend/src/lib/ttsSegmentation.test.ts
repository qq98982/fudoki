import { expect, test } from 'vitest'

import { normalizeTextForRemoteTts, splitTextByPunctuation } from './ttsSegmentation'

test('normalizes remote tts text by removing parenthetical content', () => {
  expect(normalizeTextForRemoteTts('第1条（目的）本契約は、甲乙間の条件を定める。')).toBe(
    '第1条本契約は 甲乙間の条件を定める。',
  )
})

test('normalizes Japanese commas to ASCII commas for lighter remote pauses', () => {
  expect(normalizeTextForRemoteTts('本契約は、甲乙間の条件を定める。')).toBe(
    '本契約は 甲乙間の条件を定める。',
  )
})

test('does not add extra pause for punctuation-delimited segments', () => {
  expect(splitTextByPunctuation('第一文です。第二文です。')).toEqual([
    { text: '第一文です。', pause: 0 },
    { text: '第二文です。', pause: 0 },
  ])
})

test('keeps a short pause for line breaks after normalization', () => {
  expect(splitTextByPunctuation('見出し\n本文です。')[0]).toEqual({
    text: '見出し',
    pause: 180,
  })
})

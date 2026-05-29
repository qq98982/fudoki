import { render, screen } from '@testing-library/react'
import { expect, test, vi } from 'vitest'

import { AnalysisStrip } from './AnalysisStrip'

test('filters markdown markers and renders punctuation without readings or explanations', () => {
  render(
    <AnalysisStrip
      analysis={{
        lines: [
          [
            {
              surface: '#',
              lemma: '#',
              reading: '#',
              tts_text: '#',
              pos: ['補助記号'],
              source: 'sudachi',
              confidence: 1,
            },
            {
              surface: ' ',
              lemma: ' ',
              reading: '',
              tts_text: ' ',
              pos: ['空白'],
              source: 'sudachi',
              confidence: 1,
            },
            {
              surface: '(',
              lemma: '(',
              reading: 'カッコ',
              tts_text: '(',
              pos: ['補助記号'],
              source: 'sudachi',
              confidence: 1,
            },
            {
              surface: '契約条項',
              lemma: '契約条項',
              reading: 'ケイヤクジョウコウ',
              tts_text: 'ケイヤクジョウコウ',
              pos: ['名詞'],
              source: 'sudachi',
              confidence: 1,
            },
            {
              surface: ')',
              lemma: ')',
              reading: 'カッコ',
              tts_text: ')',
              pos: ['補助記号'],
              source: 'sudachi',
              confidence: 1,
            },
            {
              surface: '。',
              lemma: '。',
              reading: 'マル',
              tts_text: '。',
              pos: ['補助記号'],
              source: 'sudachi',
              confidence: 1,
            },
          ],
        ],
      }}
      analysisSource="fresh"
      analysisStatus="ready"
      lang="ja"
      onPlayLine={vi.fn()}
      onTokenSelect={vi.fn()}
      selectedToken={null}
      showKana
      showPos
    />,
  )

  expect(screen.queryByText('#')).not.toBeInTheDocument()
  expect(screen.queryByText('契約条項')).not.toBeInTheDocument()
  expect(screen.queryByText('(')).not.toBeInTheDocument()
  expect(screen.queryByText(')')).not.toBeInTheDocument()
  expect(screen.queryByText('空白')).not.toBeInTheDocument()
  expect(screen.getByText('。')).toBeInTheDocument()
  expect(screen.queryByText('カッコ')).not.toBeInTheDocument()
  expect(screen.queryByText('マル')).not.toBeInTheDocument()
  expect(screen.getAllByRole('button').some((button) => button.textContent?.includes('契約条項'))).toBe(false)
})

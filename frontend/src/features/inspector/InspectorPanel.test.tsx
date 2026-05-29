import { render, screen } from '@testing-library/react'
import { expect, test, vi } from 'vitest'

import { InspectorPanel } from './InspectorPanel'

test('renders dictionary payload returned by backend without crashing', () => {
  render(
    <InspectorPanel
      activeTab="dictionary"
      analysis={null}
      analysisSource={null}
      analysisStatus="idle"
      analysisCacheStatus="idle"
      currentProviderId="system"
      dictionaryEntry={{
        word: 'ウェブ',
        query: 'ウェブ',
        kanji: [],
        kana: [{ text: 'ウェブ', common: true }],
        senses: [
          {
            gloss: 'web; World Wide Web',
            partOfSpeech: ['n'],
            field: [],
            misc: [],
            info: [],
          },
        ],
        lookupSource: 'jmdict',
      }}
      lang="ja"
      onLanguageChange={vi.fn()}
      onClearAnalysisCache={vi.fn()}
      onPlay={vi.fn()}
      onProviderChange={vi.fn()}
      onStop={vi.fn()}
      onTabChange={vi.fn()}
      onThemeChange={vi.fn()}
      onToggleSetting={vi.fn()}
      providers={undefined}
      selectedToken={{
        surface: 'ウェブ',
        lemma: 'ウェブ',
        reading: 'ウェブ',
        tts_text: 'ウェブ',
        pos: ['名詞'],
        source: 'sudachi',
        confidence: 1,
      }}
      showKana
      showPos
      theme="paper"
    />,
  )

  expect(screen.getByRole('heading', { name: '辞書' })).toBeInTheDocument()
  expect(screen.getAllByText('ウェブ')).toHaveLength(2)
  expect(screen.getByText('web; World Wide Web')).toBeInTheDocument()
  expect(screen.getByText('n')).toBeInTheDocument()
})

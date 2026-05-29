import { t } from '../../lib/i18n'
import type { AnalyzeResponse, AnalyzeToken } from '../../types'
import { lineSpeechText, toDisplayItems, tokenPosKey, tokenRomaji } from './analysisTokens'

interface AnalysisStripProps {
  lang: string
  analysis: AnalyzeResponse | null
  analysisStatus: 'idle' | 'loading' | 'ready' | 'stale' | 'error'
  analysisSource: 'cached' | 'fresh' | null
  showKana: boolean
  showPos: boolean
  selectedToken: AnalyzeToken | null
  onTokenSelect: (token: AnalyzeToken) => void
  onPlayLine: (text: string) => void
}

export function AnalysisStrip({
  lang,
  analysis,
  analysisStatus,
  analysisSource,
  showKana,
  showPos,
  selectedToken,
  onTokenSelect,
  onPlayLine,
}: AnalysisStripProps) {
  return (
    <section className="analysis-strip" aria-label="Inline analysis strip">
      <header className="analysis-strip__header">
        <div>
          <p className="eyebrow">Inline Analysis</p>
          <strong>{analysisHeaderText(lang, analysisStatus, analysisSource)}</strong>
        </div>
      </header>

      {!analysis ? (
        <p className="muted analysis-strip__empty">{t(lang, 'analysisEmpty')}</p>
      ) : (
        <div className="analysis-strip__lines">
          {analysis.lines.map((line, lineIndex) => {
            const displayItems = toDisplayItems(line)
            const speakableText = lineSpeechText(line)

            if (displayItems.length === 0) {
              return null
            }

            return (
              <div className="analysis-line" key={`line-${lineIndex}`}>
                <div className="analysis-line__tokens">
                  {displayItems.map((item, tokenIndex) => {
                    if (item.kind === 'punct') {
                      return (
                        <span className="analysis-punct" key={`${lineIndex}-${tokenIndex}-${item.surface}`}>
                          {item.surface}
                        </span>
                      )
                    }

                    const token = item.token
                    const posKey = tokenPosKey(token)
                    const isSelected =
                      selectedToken?.surface === token.surface &&
                      selectedToken?.reading === token.reading &&
                      selectedToken?.lemma === token.lemma

                    return (
                      <button
                        className={`analysis-token analysis-token--${posKey} ${isSelected ? 'active' : ''}`}
                        key={`${lineIndex}-${tokenIndex}-${token.surface}`}
                        onClick={() => onTokenSelect(token)}
                        type="button"
                      >
                        <span className="analysis-token__top">
                          {showKana ? <span className="analysis-token__kana">{token.reading}</span> : null}
                          <span className="analysis-token__romaji">{tokenRomaji(token.reading)}</span>
                        </span>
                        <span className="analysis-token__surface">{token.surface}</span>
                        {showPos ? <span className="analysis-token__pos">{token.pos[0] ?? ''}</span> : null}
                      </button>
                    )
                  })}
                </div>
                {speakableText ? (
                  <button
                    aria-label={`Play line ${lineIndex + 1}`}
                    className="analysis-line__play"
                    onClick={() => onPlayLine(speakableText)}
                    type="button"
                  >
                    ▶
                  </button>
                ) : null}
              </div>
            )
          })}
        </div>
      )}
    </section>
  )
}

function analysisHeaderText(
  lang: string,
  status: 'idle' | 'loading' | 'ready' | 'stale' | 'error',
  source: 'cached' | 'fresh' | null,
) {
  if (status === 'loading') return t(lang, 'analysisLoading')
  if (status === 'stale') return t(lang, 'analysisStale')
  if (status === 'error') return t(lang, 'analysisError')
  if (status === 'ready' && source === 'cached') return t(lang, 'analysisCached')
  if (status === 'ready' && source === 'fresh') return t(lang, 'analysisFresh')
  return t(lang, 'analysisEmpty')
}

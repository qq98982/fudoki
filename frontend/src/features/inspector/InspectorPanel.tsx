import { t } from '../../lib/i18n'
import type {
  AnalyzeResponse,
  AnalyzeToken,
  DictionaryEntry,
  InspectorTab,
  TtsProvidersResponse,
} from '../../types'

interface InspectorPanelProps {
  lang: string
  activeTab: InspectorTab
  analysis: AnalyzeResponse | null
  analysisStatus: 'idle' | 'loading' | 'ready' | 'stale' | 'error'
  analysisSource: 'cached' | 'fresh' | null
  selectedToken: AnalyzeToken | null
  dictionaryEntry: DictionaryEntry | null
  providers: TtsProvidersResponse | undefined
  currentProviderId: string
  showKana: boolean
  showPos: boolean
  theme: string
  analysisCacheStatus: 'idle' | 'success' | 'error'
  onTabChange: (tab: InspectorTab) => void
  onPlay: () => void
  onStop: () => void
  onProviderChange: (value: string) => void
  onThemeChange: (value: string) => void
  onLanguageChange: (value: string) => void
  onToggleSetting: (key: 'showKana' | 'showPos', value: boolean) => void
  onClearAnalysisCache: () => void
}

const tabs: InspectorTab[] = ['analysis', 'dictionary', 'tts', 'settings']

export function InspectorPanel(props: InspectorPanelProps) {
  const {
    lang,
    activeTab,
    analysis,
    analysisStatus,
    analysisSource,
    selectedToken,
    dictionaryEntry,
    providers,
    currentProviderId,
    showKana,
    showPos,
    theme,
    analysisCacheStatus,
    onTabChange,
    onPlay,
    onStop,
    onProviderChange,
    onThemeChange,
    onLanguageChange,
    onToggleSetting,
    onClearAnalysisCache,
  } = props

  return (
    <aside className="inspector-panel">
      <div className="tab-row">
        {tabs.map((tab) => (
          <button
            className={tab === activeTab ? 'tab-button active' : 'tab-button'}
            key={tab}
            onClick={() => onTabChange(tab)}
            type="button"
          >
            {t(lang, tab)}
          </button>
        ))}
      </div>

      {activeTab === 'analysis' ? (
        <section className="panel-section">
          <h2>{t(lang, 'analysis')}</h2>
          <p className="muted">
            {analysisStatus === 'loading'
              ? t(lang, 'analysisLoading')
              : analysisStatus === 'stale'
                ? t(lang, 'analysisStale')
                : analysisStatus === 'error'
                  ? t(lang, 'analysisError')
                  : analysisSource === 'cached'
                    ? t(lang, 'analysisCached')
                    : analysisSource === 'fresh'
                      ? t(lang, 'analysisFresh')
                      : t(lang, 'analysisEmpty')}
          </p>
          {!analysis ? (
            <p className="muted">{t(lang, 'analysisEmpty')}</p>
          ) : selectedToken ? (
            <div className="dictionary-card">
              <strong>{selectedToken.surface}</strong>
              <p>{selectedToken.reading}</p>
              <small>{selectedToken.pos.join(', ')}</small>
            </div>
          ) : (
            <p className="muted">{t(lang, 'dictionaryEmpty')}</p>
          )}
        </section>
      ) : null}

      {activeTab === 'dictionary' ? (
        <section className="panel-section">
          <h2>{t(lang, 'dictionary')}</h2>
          {!dictionaryEntry ? (
            <p className="muted">{t(lang, 'dictionaryEmpty')}</p>
          ) : (
            <div className="dictionary-card">
              <strong>{dictionaryEntry.word ?? dictionaryEntry.query ?? selectedToken?.surface}</strong>
              <p>{(dictionaryEntry.kana ?? []).map((item) => item.text).join(' / ')}</p>
              {(dictionaryEntry.senses ?? []).map((sense, index) => (
                <article className="sense-card" key={`${index}-${sense.gloss ?? 'sense'}`}>
                  <p>{sense.gloss ?? ''}</p>
                  <small>{(sense.partOfSpeech ?? []).join(', ')}</small>
                </article>
              ))}
            </div>
          )}
        </section>
      ) : null}

      {activeTab === 'tts' ? (
        <section className="panel-section">
          <h2>{t(lang, 'tts')}</h2>
          <p className="muted">{t(lang, 'ttsReady')}</p>
          <label className="field">
            <span>Provider</span>
            <select onChange={(event) => onProviderChange(event.target.value)} value={currentProviderId}>
              {(providers?.providers ?? [{ id: 'system', status: 'available' }]).map((provider) => (
                <option key={provider.id} value={provider.id}>
                  {provider.id}
                </option>
              ))}
            </select>
          </label>
          <div className="button-row">
            <button className="primary-button" onClick={onPlay} type="button">
              {t(lang, 'speakDocument')}
            </button>
            <button className="secondary-button" onClick={onStop} type="button">
              {t(lang, 'stopPlayback')}
            </button>
          </div>
        </section>
      ) : null}

      {activeTab === 'settings' ? (
        <section className="panel-section">
          <h2>{t(lang, 'settings')}</h2>
          <label className="field">
            <span>{t(lang, 'theme')}</span>
            <select onChange={(event) => onThemeChange(event.target.value)} value={theme}>
              <option value="paper">{t(lang, 'paper')}</option>
              <option value="dark">{t(lang, 'dark')}</option>
            </select>
          </label>
          <label className="field">
            <span>{t(lang, 'language')}</span>
            <select onChange={(event) => onLanguageChange(event.target.value)} value={lang}>
              <option value="zh">中文</option>
              <option value="ja">日本語</option>
              <option value="en">English</option>
            </select>
          </label>
          <label className="checkbox">
            <input checked={showKana} onChange={(event) => onToggleSetting('showKana', event.target.checked)} type="checkbox" />
            <span>{t(lang, 'showKana')}</span>
          </label>
          <label className="checkbox">
            <input checked={showPos} onChange={(event) => onToggleSetting('showPos', event.target.checked)} type="checkbox" />
            <span>{t(lang, 'showPos')}</span>
          </label>
          <div className="settings-tools">
            <button className="secondary-button" onClick={onClearAnalysisCache} type="button">
              {t(lang, 'clearAnalysisCache')}
            </button>
            {analysisCacheStatus === 'success' ? (
              <p className="muted settings-tools__feedback">{t(lang, 'clearAnalysisCacheSuccess')}</p>
            ) : null}
            {analysisCacheStatus === 'error' ? (
              <p className="muted settings-tools__feedback settings-tools__feedback--error">
                {t(lang, 'clearAnalysisCacheError')}
              </p>
            ) : null}
          </div>
        </section>
      ) : null}
    </aside>
  )
}

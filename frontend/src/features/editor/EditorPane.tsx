import { useCallback, type ReactNode } from 'react'
import CodeMirror from '@uiw/react-codemirror'
import { EditorView } from '@codemirror/view'

import { t } from '../../lib/i18n'

interface EditorPaneProps {
  lang: string
  draft: string
  saveState: 'idle' | 'saving' | 'conflict' | 'error'
  children?: ReactNode
  onChange: (value: string) => void
  onAnalyze: () => void
}

export function EditorPane({ lang, draft, saveState, children, onChange, onAnalyze }: EditorPaneProps) {
  const saveLabel =
    saveState === 'saving'
      ? t(lang, 'savePending')
      : saveState === 'conflict'
        ? t(lang, 'saveConflict')
        : saveState === 'error'
          ? t(lang, 'saveError')
          : t(lang, 'saveIdle')

  const handleChange = useCallback(
    (value: string) => {
      onChange(value)
    },
    [onChange],
  )

  return (
    <section className="editor-pane">
      <header className="editor-toolbar">
        <div>
          <p className="eyebrow">Article</p>
          <p className="save-state">{saveLabel}</p>
        </div>
        <button className="primary-button" onClick={onAnalyze} type="button">
          {t(lang, 'analyze')}
        </button>
      </header>

      <CodeMirror
        aria-label="Document editor"
        className="editor-input"
        extensions={[EditorView.lineWrapping]}
        onChange={handleChange}
        placeholder={t(lang, 'editorPlaceholder')}
        value={draft}
      />

      {children}
    </section>
  )
}

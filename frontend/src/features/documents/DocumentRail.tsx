import { useDeferredValue } from 'react'

import { t } from '../../lib/i18n'
import type { DocumentRecord } from '../../types'

interface DocumentRailProps {
  lang: string
  documents: DocumentRecord[]
  activeDocumentId: string | null
  search: string
  onSearchChange: (value: string) => void
  onCreate: () => void
  onSelect: (id: string) => void
  onRename: (document: DocumentRecord) => void
  onDelete: (document: DocumentRecord) => void
  onDuplicate: (document: DocumentRecord) => void
}

export function DocumentRail({
  lang,
  documents,
  activeDocumentId,
  search,
  onSearchChange,
  onCreate,
  onSelect,
  onRename,
  onDelete,
  onDuplicate,
}: DocumentRailProps) {
  const deferredSearch = useDeferredValue(search.trim().toLowerCase())
  const filtered = documents.filter((document) => {
    if (!deferredSearch) {
      return true
    }
    return [document.title, document.content].join('\n').toLowerCase().includes(deferredSearch)
  })

  return (
    <aside className="document-rail">
      <div className="rail-header">
        <div>
          <p className="eyebrow">{t(lang, 'documents')}</p>
          <h1>{t(lang, 'workspaceTitle')}</h1>
          <p className="subtitle">{t(lang, 'workspaceSubtitle')}</p>
        </div>
        <button className="primary-button" onClick={onCreate} type="button">
          {t(lang, 'newDocument')}
        </button>
      </div>

      <label className="search-label">
        <span className="sr-only">{t(lang, 'searchDocuments')}</span>
        <input
          aria-label={t(lang, 'searchDocuments')}
          className="search-input"
          onChange={(event) => onSearchChange(event.target.value)}
          placeholder={t(lang, 'searchDocuments')}
          value={search}
        />
      </label>

      <div className="document-list">
        {filtered.length === 0 ? (
          <div className="empty-card">{t(lang, 'noDocuments')}</div>
        ) : (
          filtered.map((document) => (
            <article
              className={`document-card ${document.id === activeDocumentId ? 'active' : ''}`}
              key={document.id}
            >
              <button className="document-main" onClick={() => onSelect(document.id)} type="button">
                <strong>{document.title}</strong>
                <span>{new Date(document.updated_at).toLocaleString()}</span>
              </button>
              <div className="document-actions">
                <button onClick={() => onRename(document)} type="button">
                  {t(lang, 'rename')}
                </button>
                <button onClick={() => onDuplicate(document)} type="button">
                  {t(lang, 'duplicate')}
                </button>
                <button className="danger" onClick={() => onDelete(document)} type="button">
                  {t(lang, 'delete')}
                </button>
              </div>
            </article>
          ))
        )}
      </div>
    </aside>
  )
}

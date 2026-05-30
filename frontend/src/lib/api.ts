import type {
  AnalyzeResponse,
  DictionaryEntry,
  DocumentEnvelope,
  DocumentsResponse,
  SettingsResponse,
  TtsProvidersResponse,
  TitleMode,
} from '../types'

export class ApiError extends Error {
  code: string
  status: number

  constructor(code: string, message: string, status: number) {
    super(message)
    this.name = 'ApiError'
    this.code = code
    this.status = status
  }
}

async function requestJson<T>(input: RequestInfo, init?: RequestInit): Promise<T> {
  const response = await fetch(input, {
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers ?? {}),
    },
    ...init,
  })

  if (!response.ok) {
    try {
      const body = await response.json()
      if (body?.error?.code && body?.error?.message) {
        throw new ApiError(body.error.code, body.error.message, response.status)
      }
    } catch (e) {
      if (e instanceof ApiError) throw e
    }
    const text = await response.text()
    throw new Error(text || `Request failed: ${response.status}`)
  }

  return (await response.json()) as T
}

export async function checkHealth() {
  const response = await fetch('/api/health')
  if (!response.ok) {
    throw new Error('Backend unavailable')
  }
  return (await response.json()) as { status: string; tokenizer_ready: boolean; dictionary_ready: boolean }
}

export async function duplicateDocument(id: string) {
  return requestJson<DocumentEnvelope>(`/api/documents/${id}/duplicate`, { method: 'POST' })
}

export async function listDocuments() {
  return requestJson<DocumentsResponse>('/api/documents')
}

export async function createDocument(payload: {
  content: string
  title_mode?: TitleMode
  title?: string
}) {
  return requestJson<DocumentEnvelope>('/api/documents', {
    method: 'POST',
    body: JSON.stringify(payload),
  })
}

export async function updateDocument(
  id: string,
  payload: {
    content: string
    expected_revision: number
    title_mode?: TitleMode
    title?: string
  },
) {
  return requestJson<DocumentEnvelope>(`/api/documents/${id}`, {
    method: 'PUT',
    body: JSON.stringify(payload),
  })
}

export async function deleteDocument(id: string) {
  const response = await fetch(`/api/documents/${id}`, { method: 'DELETE' })
  if (!response.ok) {
    const text = await response.text()
    throw new Error(text || `Delete failed: ${response.status}`)
  }
}

export async function listSettings() {
  return requestJson<SettingsResponse>('/api/settings')
}

export async function updateSettings(values: Record<string, unknown>) {
  return requestJson<SettingsResponse>('/api/settings', {
    method: 'PUT',
    body: JSON.stringify({ values }),
  })
}

export async function analyzeDocument(payload: {
  text: string
  document_id?: string
  document_revision?: number
}) {
  return requestJson<AnalyzeResponse>('/api/analyze', {
    method: 'POST',
    body: JSON.stringify(payload),
  })
}

export async function getCachedAnalysis(documentId: string, revision: number) {
  const response = await fetch(`/api/documents/${documentId}/analysis?revision=${revision}`)
  if (response.status === 404) {
    return null
  }
  if (!response.ok) {
    const text = await response.text()
    throw new Error(text || `Cached analysis failed: ${response.status}`)
  }
  return (await response.json()) as AnalyzeResponse
}

export async function clearAnalysisCache() {
  const response = await fetch('/api/analysis-cache', { method: 'DELETE' })
  if (!response.ok && response.status !== 204) {
    const text = await response.text()
    throw new Error(text || `Clear analysis cache failed: ${response.status}`)
  }
}

export async function lookupDictionary(term: string) {
  const response = await fetch(`/api/dictionary?term=${encodeURIComponent(term)}`)
  if (response.status === 404) {
    return null
  }
  if (!response.ok) {
    const text = await response.text()
    throw new Error(text || `Dictionary failed: ${response.status}`)
  }
  return (await response.json()) as DictionaryEntry
}

export async function listTtsProviders() {
  return requestJson<TtsProvidersResponse>('/api/tts/providers')
}

export async function requestRemoteSpeech(payload: {
  provider: string
  text: string
  model?: string
  voice?: string
  format?: string
  speed?: number
  document_id?: string
  document_revision?: number
  cache_scope_version?: string
}) {
  const response = await fetch('/api/tts/speak', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  })
  if (!response.ok) {
    const text = await response.text()
    throw new Error(text || `TTS failed: ${response.status}`)
  }
  return response
}

export async function importLegacyBrowserData(payload: {
  documents: Array<{
    id: string
    content: string | string[]
    createdAt?: number
    updatedAt?: number
  }>
  activeId?: string | null
  settings: Record<string, unknown>
}) {
  return requestJson<{ imported_documents: number }>('/api/migrations/legacy-browser-data', {
    method: 'POST',
    body: JSON.stringify(payload),
  })
}

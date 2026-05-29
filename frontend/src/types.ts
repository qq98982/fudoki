export type TitleMode = 'auto' | 'custom'

export interface DocumentRecord {
  id: string
  title: string
  title_mode: TitleMode
  content: string
  source_kind: string
  created_at: number
  updated_at: number
  revision: number
}

export interface DocumentsResponse {
  documents: DocumentRecord[]
  active_document_id: string | null
}

export interface DocumentEnvelope {
  document: DocumentRecord
}

export interface SettingsResponse {
  values: Record<string, unknown>
}

export interface AnalyzeToken {
  surface: string
  lemma: string
  reading: string
  tts_text: string
  pos: string[]
  source: string
  confidence: number
}

export interface AnalyzeResponse {
  lines: AnalyzeToken[][]
}

export interface DictionarySense {
  gloss?: string
  partOfSpeech?: string[]
  field?: string[]
  misc?: string[]
  info?: string[]
  chineseSource?: string | null
}

export interface DictionaryTextValue {
  text: string
  common?: boolean
}

export interface DictionaryEntry {
  word?: string
  query?: string
  kanji?: DictionaryTextValue[]
  kana?: DictionaryTextValue[]
  senses?: DictionarySense[]
  lookupSource?: string
  hasMultipleMeanings?: boolean
  totalResults?: number
}

export interface TtsProvider {
  id: string
  status: string
  defaults?: {
    model: string
    voice: string
    format: string
  }
  options?: {
    models: string[]
    voices: string[]
  }
}

export interface TtsProvidersResponse {
  default_provider: string
  providers: TtsProvider[]
}

export type InspectorTab = 'analysis' | 'dictionary' | 'tts' | 'settings'

import {
  QueryClient,
  QueryClientProvider,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query'
import { startTransition, useEffect, useMemo, useRef, useState } from 'react'

import { useAppStore } from './app/store'
import { AnalysisStrip } from './features/analysis/AnalysisStrip'
import { DocumentRail } from './features/documents/DocumentRail'
import { EditorPane } from './features/editor/EditorPane'
import { InspectorPanel } from './features/inspector/InspectorPanel'
import {
  analyzeDocument,
  checkHealth,
  clearAnalysisCache,
  createDocument,
  deleteDocument as deleteDocumentRequest,
  duplicateDocument,
  getCachedAnalysis,
  importLegacyBrowserData,
  listDocuments,
  listSettings,
  listTtsProviders,
  lookupDictionary,
  requestRemoteSpeech,
  updateDocument,
  updateSettings,
} from './lib/api'
import { t } from './lib/i18n'
import { splitTextByPunctuation } from './lib/ttsSegmentation'
import { speakWithSystemSpeech, stopSystemSpeech } from './lib/systemTts'
import type { AnalyzeResponse, DocumentRecord } from './types'

type SaveState = 'idle' | 'saving' | 'conflict' | 'error'
type AnalysisState = 'idle' | 'loading' | 'ready' | 'stale' | 'error'
type PlaybackStatus = 'idle' | 'loading' | 'playing'
type RenameDialogState = { document: DocumentRecord; title: string } | null
type DeleteDialogState = { document: DocumentRecord } | null

const REMOTE_SYNTHESIS_SPEED = 1

export default function App() {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            retry: false,
            staleTime: 5_000,
          },
        },
      }),
  )

  return (
    <QueryClientProvider client={queryClient}>
      <WorkspaceApp />
    </QueryClientProvider>
  )
}

function WorkspaceApp() {
  const queryClient = useQueryClient()
  const {
    activeDocumentId,
    inspectorTab,
    search,
    selectedToken,
    setActiveDocumentId,
    setInspectorTab,
    setSearch,
    setSelectedToken,
  } = useAppStore()

  const documentsQuery = useQuery({
    queryKey: ['documents'],
    queryFn: listDocuments,
  })
  const settingsQuery = useQuery({
    queryKey: ['settings'],
    queryFn: listSettings,
  })
  const providersQuery = useQuery({
    queryKey: ['tts-providers'],
    queryFn: listTtsProviders,
  })
  const healthQuery = useQuery({
    queryKey: ['health'],
    queryFn: checkHealth,
    retry: true,
    retryDelay: (attempt) => Math.min(1000 * 2 ** attempt, 10_000),
  })

  const [draft, setDraft] = useState('')
  const [draftRevision, setDraftRevision] = useState(0)
  const [saveState, setSaveState] = useState<SaveState>('idle')
  const [analysis, setAnalysis] = useState<AnalyzeResponse | null>(null)
  const [analysisState, setAnalysisState] = useState<AnalysisState>('idle')
  const [analysisSource, setAnalysisSource] = useState<'cached' | 'fresh' | null>(null)
  const [analysisCacheStatus, setAnalysisCacheStatus] = useState<'idle' | 'success' | 'error'>('idle')
  const [playbackStatus, setPlaybackStatus] = useState<PlaybackStatus>('idle')
  const [playbackSpeed, setPlaybackSpeed] = useState(1)
  const [renameDialog, setRenameDialog] = useState<RenameDialogState>(null)
  const [deleteDialog, setDeleteDialog] = useState<DeleteDialogState>(null)
  const [operationMessage, setOperationMessage] = useState<string | null>(null)

  const savePromiseRef = useRef<Promise<void> | null>(null)
  const audioRef = useRef<HTMLAudioElement | null>(null)
  const currentAudioUrlRef = useRef<string | null>(null)
  const playbackSessionRef = useRef(0)
  const playbackControllerRef = useRef<{ stop: () => void } | null>(null)
  const playbackSpeedRef = useRef(1)
  const bootstrappedLegacyRef = useRef(false)
  const syncedDocumentIdRef = useRef<string | null>(null)
  const suppressNextAutoAnalyzeRef = useRef(false)
  const lastAnalyzedTextRef = useRef('')
  const lastAnalyzedSignatureRef = useRef('')

  const settingsMutation = useMutation({
    mutationFn: updateSettings,
    onSuccess(data) {
      queryClient.setQueryData(['settings'], data)
    },
  })

  const renameMutation = useMutation({
    mutationFn: ({
      id,
      title,
      titleMode,
      content,
      expectedRevision,
    }: {
      id: string
      title: string
      titleMode: string
      content: string
      expectedRevision: number
    }) =>
      updateDocument(id, {
        title,
        title_mode: titleMode === 'auto' ? 'auto' : 'custom',
        content,
        expected_revision: expectedRevision,
      }),
  })

  const dictionaryQuery = useQuery({
    queryKey: ['dictionary', selectedToken?.surface],
    queryFn: () => lookupDictionary(selectedToken?.surface ?? ''),
    enabled: Boolean(selectedToken?.surface),
  })

  const documents = useMemo(() => documentsQuery.data?.documents ?? [], [documentsQuery.data?.documents])
  const settings = settingsQuery.data?.values ?? {}
  const lang = typeof settings.lang === 'string' ? settings.lang : 'zh'
  const theme = settings.theme === 'dark' ? 'dark' : 'paper'
  const showKana = settings.showKana !== false
  const showPos = settings.showPos !== false
  const settingsPlaybackSpeed = normalizePlaybackSpeed(settings.ttsPlaybackSpeed)
  const currentProviderId =
    typeof settings.ttsProvider === 'string'
      ? settings.ttsProvider
      : providersQuery.data?.default_provider ?? 'system'

  const activeDocument =
    documents.find((document) => document.id === activeDocumentId) ??
    (activeDocumentId ? null : documents[0] ?? null)

  async function persistDraft() {
    if (!activeDocument) {
      return
    }

    if (savePromiseRef.current) {
      return savePromiseRef.current
    }

    if (draft === activeDocument.content) {
      return
    }

    setSaveState('saving')

    const savePromise = updateDocument(activeDocument.id, {
      content: draft,
      expected_revision: draftRevision,
      title_mode: activeDocument.title_mode,
      title: activeDocument.title_mode === 'custom' ? activeDocument.title : undefined,
    })
      .then(({ document }) => {
        setDraftRevision(document.revision)
        setSaveState('idle')
        queryClient.setQueryData(['documents'], (current: DocumentsLike) =>
          updateDocumentsCache(current, document),
        )
      })
      .catch(async (error: unknown) => {
        const message = String(error ?? '')
        if (message.includes('409') || message.includes('document_revision_conflict')) {
          const latest = await documentsQuery.refetch()
          const freshDocument = latest.data?.documents.find((document) => document.id === activeDocument.id)
          if (freshDocument) {
            setDraftRevision(freshDocument.revision)
          }
          setSaveState('conflict')
          return
        }

        setSaveState('error')
      })
      .finally(() => {
        savePromiseRef.current = null
      })

    savePromiseRef.current = savePromise
    return savePromise
  }

  async function handleCreateDocument() {
    await persistDraft()
    const created = await createDocument({ content: '', title_mode: 'auto' })
    queryClient.setQueryData(['documents'], (current: DocumentsLike) => {
      const currentValue = ensureDocumentsShape(current)
      return {
        documents: [created.document, ...currentValue.documents],
        active_document_id: created.document.id,
      }
    })
    startTransition(() => {
      setActiveDocumentId(created.document.id)
      setInspectorTab('analysis')
    })
  }

  async function handleSelectDocument(id: string) {
    if (id === activeDocumentId) {
      return
    }
    await persistDraft()
    startTransition(() => {
      setActiveDocumentId(id)
      setInspectorTab('analysis')
    })
  }

  function showOperationMessage(message: string) {
    setOperationMessage(message)
  }

  function requestRenameDocument(document: DocumentRecord) {
    setRenameDialog({ document, title: document.title })
  }

  async function confirmRenameDocument() {
    if (!renameDialog) {
      return
    }

    const document = renameDialog.document
    const nextTitle = renameDialog.title.trim()
    if (!nextTitle) {
      showOperationMessage(t(lang, 'renameTitleRequired'))
      return
    }

    try {
      const content = document.id === activeDocumentId ? draft : document.content
      const expectedRevision = document.id === activeDocumentId ? draftRevision : document.revision
      const updated = await renameMutation.mutateAsync({
        id: document.id,
        title: nextTitle,
        titleMode: 'custom',
        content,
        expectedRevision,
      })

      queryClient.setQueryData(['documents'], (current: DocumentsLike) =>
        updateDocumentsCache(current, updated.document),
      )

      if (document.id === activeDocumentId) {
        setDraftRevision(updated.document.revision)
      }
      setRenameDialog(null)
    } catch {
      showOperationMessage(t(lang, 'renameFailed'))
    }
  }

  async function handleDuplicateDocument(document: DocumentRecord) {
    await persistDraft()
    try {
      const duplicated = await duplicateDocument(document.id)
      queryClient.setQueryData(['documents'], (current: DocumentsLike) => {
        const currentValue = ensureDocumentsShape(current)
        return {
          documents: [duplicated.document, ...currentValue.documents],
          active_document_id: duplicated.document.id,
        }
      })
      startTransition(() => {
        setActiveDocumentId(duplicated.document.id)
        setInspectorTab('analysis')
      })
    } catch {
      showOperationMessage(t(lang, 'duplicateFailed'))
    }
  }

  function requestDeleteDocument(document: DocumentRecord) {
    setDeleteDialog({ document })
  }

  async function confirmDeleteDocument() {
    if (!deleteDialog) {
      return
    }

    const document = deleteDialog.document
    try {
      await deleteDocumentRequest(document.id)
      queryClient.setQueryData(['documents'], (current: DocumentsLike) => {
        const currentValue = ensureDocumentsShape(current)
        const nextDocuments = currentValue.documents.filter((item) => item.id !== document.id)
        return {
          documents: nextDocuments,
          active_document_id:
            currentValue.active_document_id === document.id
              ? nextDocuments[0]?.id ?? null
              : currentValue.active_document_id,
        }
      })

      if (document.id === activeDocumentId) {
        setActiveDocumentId(null)
      }
      setDeleteDialog(null)
    } catch {
      showOperationMessage(t(lang, 'deleteFailed'))
    }
  }

  async function handleAnalyze() {
    return handleAnalyzeInternal('manual')
  }

  async function handleAnalyzeInternal(mode: 'manual' | 'auto') {
    if (!draft.trim()) {
      setAnalysis(null)
      setAnalysisState('idle')
      setAnalysisSource(null)
      lastAnalyzedTextRef.current = ''
      lastAnalyzedSignatureRef.current = ''
      return
    }

    if (mode === 'manual') {
      setInspectorTab('analysis')
    }

    setAnalysisState('loading')

    const payload =
      activeDocument && draft === activeDocument.content
        ? {
            text: draft,
            document_id: activeDocument.id,
            document_revision: draftRevision || undefined,
          }
        : {
            text: draft,
          }

    try {
      const result = await analyzeDocument(payload)
      lastAnalyzedTextRef.current = draft
      lastAnalyzedSignatureRef.current = computeStructureSignature(draft)
      setAnalysis(result)
      setAnalysisState('ready')
      setAnalysisSource('fresh')
    } catch {
      setAnalysisState('error')
    }
  }

  async function loadCachedAnalysis(documentId: string, revision: number, content: string) {
    try {
      const cached = await getCachedAnalysis(documentId, revision)
      if (syncedDocumentIdRef.current !== documentId) {
        return
      }

      if (!cached) {
        setAnalysis(null)
        setAnalysisState('idle')
        setAnalysisSource(null)
        lastAnalyzedTextRef.current = ''
        lastAnalyzedSignatureRef.current = ''
        if (content.trim()) {
          void handleAnalyzeInternal('auto')
        }
        return
      }

      setAnalysis(cached)
      setAnalysisState('ready')
      setAnalysisSource('cached')
      lastAnalyzedTextRef.current = content
      lastAnalyzedSignatureRef.current = computeStructureSignature(content)
    } catch {
      if (syncedDocumentIdRef.current === documentId) {
        setAnalysisState('error')
      }
    }
  }

  async function handleClearAnalysisCache() {
    if (!window.confirm(t(lang, 'clearAnalysisCacheConfirm'))) {
      return
    }

    try {
      await clearAnalysisCache()
      setAnalysisCacheStatus('success')
      if (activeDocument) {
        setAnalysis(null)
        setAnalysisState(activeDocument.content.trim() ? 'loading' : 'idle')
        setAnalysisSource(null)
        lastAnalyzedTextRef.current = ''
        lastAnalyzedSignatureRef.current = ''
      }
    } catch {
      setAnalysisCacheStatus('error')
    }
  }

  async function handlePlay() {
    if (currentProviderId === 'system') {
      return handlePlayText(draft)
    }
    return handlePlayRemoteDocument(draft)
  }

  async function handlePlayText(text: string) {
    if (!text.trim()) {
      return
    }

    stopPlayback()
    const sessionId = beginPlaybackSession()

    if (currentProviderId === 'system') {
      playbackControllerRef.current = { stop: () => stopSystemSpeech() }
      speakWithSystemSpeech(text, playbackSpeedRef.current, {
        onStart: () => {
          if (sessionId !== playbackSessionRef.current) return
          setPlaybackStatus('playing')
        },
        onEnd: () => {
          if (sessionId !== playbackSessionRef.current) return
          finishPlaybackSession(sessionId)
        },
        onError: () => {
          if (sessionId !== playbackSessionRef.current) return
          finishPlaybackSession(sessionId)
        },
      })
      return
    }

    const provider = providersQuery.data?.providers.find((item) => item.id === currentProviderId)
    const response = await requestRemoteSpeech({
      provider: currentProviderId,
      text,
      model: provider?.defaults?.model,
      voice: provider?.defaults?.voice,
      format: provider?.defaults?.format,
      speed: REMOTE_SYNTHESIS_SPEED,
      document_id: activeDocument?.id,
      document_revision: draftRevision || undefined,
      cache_scope_version: computeCacheScopeVersion(currentProviderId, providersQuery.data?.providers),
    })

    if (sessionId !== playbackSessionRef.current) {
      return
    }

    const blob = await response.blob()
    const url = URL.createObjectURL(blob)
    if (sessionId !== playbackSessionRef.current) {
      URL.revokeObjectURL(url)
      return
    }

    currentAudioUrlRef.current = url
    const audio = new Audio(url)
    applyAudioPlaybackSpeed(audio)
    audioRef.current = audio
    playbackControllerRef.current = {
      stop: () => {
        try {
          audio.pause()
        } catch {
          // Ignore pause errors during stop.
        }
      },
    }
    audio.onended = () => {
      if (sessionId !== playbackSessionRef.current) return
      finishPlaybackSession(sessionId)
    }
    audio.onerror = () => {
      if (sessionId !== playbackSessionRef.current) return
      finishPlaybackSession(sessionId)
    }

    await audio.play()
    if (sessionId !== playbackSessionRef.current) {
      cleanupAudio()
      return
    }
    setPlaybackStatus('playing')
  }

  async function handlePlayRemoteDocument(text: string) {
    if (!text.trim()) {
      return
    }

    stopPlayback()
    const sessionId = beginPlaybackSession()
    const segments = splitTextByPunctuation(text)
      .map((segment) => ({ text: segment.text.trim(), pause: segment.pause }))
      .filter((segment) => segment.text.length > 0)

    if (segments.length === 0) {
      finishPlaybackSession(sessionId)
      return
    }

    const provider = providersQuery.data?.providers.find((item) => item.id === currentProviderId)

    for (let index = 0; index < segments.length; index += 1) {
      const segment = segments[index]
      if (sessionId !== playbackSessionRef.current) {
        return
      }

      const response = await requestRemoteSpeech({
        provider: currentProviderId,
        text: segment.text,
        model: provider?.defaults?.model,
        voice: provider?.defaults?.voice,
        format: provider?.defaults?.format,
        speed: REMOTE_SYNTHESIS_SPEED,
        document_id: activeDocument?.id,
        document_revision: draftRevision || undefined,
        cache_scope_version: computeCacheScopeVersion(currentProviderId, providersQuery.data?.providers),
      })

      if (sessionId !== playbackSessionRef.current) {
        return
      }

      const blob = await response.blob()
      const url = URL.createObjectURL(blob)
      if (sessionId !== playbackSessionRef.current) {
        URL.revokeObjectURL(url)
        return
      }

      const segmentResult = await playRemoteAudioSegment(url, sessionId)
      if (segmentResult === 'stopped' || sessionId !== playbackSessionRef.current) {
        return
      }

      if (index < segments.length - 1 && segment.pause > 0) {
        const pauseCompleted = await waitForSegmentPause(segment.pause, sessionId)
        if (!pauseCompleted) {
          return
        }
      }
    }

    finishPlaybackSession(sessionId)
  }

  async function playRemoteAudioSegment(url: string, sessionId: number): Promise<'ended' | 'stopped'> {
    currentAudioUrlRef.current = url
    const audio = new Audio(url)
    applyAudioPlaybackSpeed(audio)
    audioRef.current = audio
    return await new Promise<'ended' | 'stopped'>((resolve, reject) => {
      let settled = false
      const settle = (result: 'ended' | 'stopped') => {
        if (settled) {
          return
        }
        settled = true
        resolve(result)
      }

      playbackControllerRef.current = {
        stop: () => {
          try {
            audio.pause()
          } catch {
            // Ignore pause errors during stop.
          }
          cleanupAudio()
          settle('stopped')
        },
      }

      audio.onended = () => {
        cleanupAudio()
        settle('ended')
      }
      audio.onerror = () => {
        cleanupAudio()
        reject(new Error('Audio playback failed'))
      }
      void audio.play().then(() => {
        if (sessionId === playbackSessionRef.current) {
          setPlaybackStatus('playing')
        }
      }, reject)
    })
  }

  async function waitForSegmentPause(pauseMs: number, sessionId: number) {
    if (pauseMs <= 0) {
      return true
    }

    return await new Promise<boolean>((resolve) => {
      const timer = window.setTimeout(() => {
        resolve(sessionId === playbackSessionRef.current)
      }, pauseMs)

      const previousController = playbackControllerRef.current
      playbackControllerRef.current = {
        stop: () => {
          window.clearTimeout(timer)
          previousController?.stop()
          resolve(false)
        },
      }
    })
  }

  function cleanupAudio() {
    if (audioRef.current) {
      try {
        audioRef.current.pause()
      } catch {
        // Ignore pause failures during cleanup.
      }
      audioRef.current = null
    }
    if (currentAudioUrlRef.current) {
      URL.revokeObjectURL(currentAudioUrlRef.current)
      currentAudioUrlRef.current = null
    }
  }

  function applyAudioPlaybackSpeed(audio: HTMLAudioElement) {
    const speed = playbackSpeedRef.current
    audio.defaultPlaybackRate = speed
    audio.playbackRate = speed
  }

  function stopPlayback() {
    playbackSessionRef.current += 1
    playbackControllerRef.current?.stop()
    playbackControllerRef.current = null
    stopSystemSpeech()
    cleanupAudio()
    setPlaybackStatus('idle')
  }

  function beginPlaybackSession() {
    const sessionId = playbackSessionRef.current + 1
    playbackSessionRef.current = sessionId
    setPlaybackStatus('loading')
    return sessionId
  }

  function finishPlaybackSession(sessionId: number) {
    if (sessionId !== playbackSessionRef.current) {
      return
    }
    playbackControllerRef.current = null
    cleanupAudio()
    setPlaybackStatus('idle')
  }

  function updateSetting(key: string, value: unknown) {
    settingsMutation.mutate({ [key]: value })
  }

  function updatePlaybackSpeed(value: number) {
    const nextSpeed = normalizePlaybackSpeed(value)
    setPlaybackSpeed(nextSpeed)
    updateSetting('ttsPlaybackSpeed', nextSpeed)
  }

  useEffect(() => {
    document.documentElement.dataset.theme = theme
    document.documentElement.lang = lang
  }, [lang, theme])

  useEffect(() => {
    setPlaybackSpeed(settingsPlaybackSpeed)
  }, [settingsPlaybackSpeed])

  useEffect(() => {
    playbackSpeedRef.current = playbackSpeed
    if (audioRef.current) {
      applyAudioPlaybackSpeed(audioRef.current)
    }
  }, [playbackSpeed])

  useEffect(() => {
    if (documentsQuery.isLoading || bootstrappedLegacyRef.current) {
      return
    }

    bootstrappedLegacyRef.current = true

    if (documents.length > 0) {
      return
    }

    const legacyTexts = localStorage.getItem('texts')
    if (!legacyTexts) {
      return
    }

    let parsedDocuments: Array<Record<string, unknown>> = []
    try {
      parsedDocuments = JSON.parse(legacyTexts) as Array<Record<string, unknown>>
    } catch {
      return
    }

    if (!Array.isArray(parsedDocuments) || parsedDocuments.length === 0) {
      return
    }

    void importLegacyBrowserData({
      documents: parsedDocuments
        .filter((document) => typeof document.id === 'string')
        .map((document) => ({
          id: String(document.id),
          content:
            typeof document.content === 'string' || Array.isArray(document.content)
              ? (document.content as string | string[])
              : '',
          createdAt: typeof document.createdAt === 'number' ? document.createdAt : undefined,
          updatedAt: typeof document.updatedAt === 'number' ? document.updatedAt : undefined,
        })),
      activeId: localStorage.getItem('activeId'),
      settings: {
        ...(localStorage.getItem('theme') ? { theme: localStorage.getItem('theme') } : {}),
        ...(localStorage.getItem('lang') ? { lang: localStorage.getItem('lang') } : {}),
        ...(localStorage.getItem('showKana')
          ? { showKana: localStorage.getItem('showKana') === 'true' }
          : {}),
        ...(localStorage.getItem('showPos')
          ? { showPos: localStorage.getItem('showPos') === 'true' }
          : {}),
        ...(localStorage.getItem('ttsProvider')
          ? { ttsProvider: localStorage.getItem('ttsProvider') }
          : {}),
      },
    }).then(async () => {
      await Promise.all([documentsQuery.refetch(), settingsQuery.refetch()])
    })
  }, [documents.length, documentsQuery, settingsQuery])

  useEffect(() => {
    if (documents.length === 0) {
      if (activeDocumentId !== null) {
        setActiveDocumentId(null)
      }
      return
    }

    const desiredActiveId = activeDocumentId ?? documentsQuery.data?.active_document_id ?? documents[0]?.id ?? null
    if (desiredActiveId !== activeDocumentId) {
      startTransition(() => {
        setActiveDocumentId(desiredActiveId)
      })
    }
  }, [activeDocumentId, documents, documentsQuery.data?.active_document_id, setActiveDocumentId])

  useEffect(() => {
    if (!activeDocument) {
      syncedDocumentIdRef.current = null
      startTransition(() => {
        setDraft('')
        setDraftRevision(0)
        setAnalysis(null)
        setAnalysisState('idle')
        setAnalysisSource(null)
      })
      return
    }

    if (syncedDocumentIdRef.current !== activeDocument.id) {
      syncedDocumentIdRef.current = activeDocument.id
      suppressNextAutoAnalyzeRef.current = true
      startTransition(() => {
        setDraft(activeDocument.content)
        setDraftRevision(activeDocument.revision)
        setSaveState('idle')
        setAnalysis(null)
        setAnalysisState(activeDocument.content.trim() ? 'loading' : 'idle')
        setAnalysisSource(null)
      })
      const documentId = activeDocument.id
      const documentRevision = activeDocument.revision
      const documentContent = activeDocument.content
      queueMicrotask(() => {
        if (syncedDocumentIdRef.current === documentId) {
          void loadCachedAnalysis(documentId, documentRevision, documentContent)
        }
      })
    }
  }, [activeDocument, setActiveDocumentId])

  useEffect(() => {
    if (!activeDocument) {
      return
    }

    if (suppressNextAutoAnalyzeRef.current) {
      suppressNextAutoAnalyzeRef.current = false
      return
    }

    if (!draft.trim()) {
      startTransition(() => {
        setAnalysis(null)
        setAnalysisState('idle')
        setAnalysisSource(null)
      })
      lastAnalyzedTextRef.current = ''
      lastAnalyzedSignatureRef.current = ''
      return
    }

    const currentSignature = computeStructureSignature(draft)
    if (currentSignature === lastAnalyzedSignatureRef.current) {
      return
    }

    lastAnalyzedSignatureRef.current = currentSignature
    startTransition(() => {
      setAnalysisState((current) => (current === 'ready' ? 'stale' : 'loading'))
    })
    const timer = window.setTimeout(() => {
      void handleAnalyzeInternal('auto')
    }, 1200)

    return () => window.clearTimeout(timer)
  }, [activeDocument, draft])

  useEffect(() => {
    if (!activeDocument) {
      return
    }

    if (draft === activeDocument.content) {
      return
    }

    const timer = window.setTimeout(() => {
      void persistDraft()
    }, 700)

    return () => window.clearTimeout(timer)
  }, [activeDocument, draft, draftRevision])

  useEffect(() => {
    return () => {
      stopPlayback()
    }
  }, [])

  return (
    <main className="workspace-shell">
      <DocumentRail
        activeDocumentId={activeDocumentId}
        documents={documents}
        lang={lang}
        onCreate={() => {
          void handleCreateDocument()
        }}
        onDelete={(document) => {
          requestDeleteDocument(document)
        }}
        onDuplicate={(document) => {
          void handleDuplicateDocument(document)
        }}
        onRename={(document) => {
          requestRenameDocument(document)
        }}
        onSearchChange={setSearch}
        onSelect={(id) => {
          void handleSelectDocument(id)
        }}
        search={search}
      />

      <section className="workspace-center">
        <EditorPane
          draft={draft}
          lang={lang}
          onAnalyze={() => {
            void handleAnalyze()
          }}
          onChange={setDraft}
          saveState={saveState}
        >
          <AnalysisStrip
            analysis={analysis}
            analysisSource={analysisSource}
            analysisStatus={analysisState}
            lang={lang}
            onPlayLine={(text) => {
              void handlePlayText(text)
            }}
            onTokenSelect={(token) => {
              setSelectedToken(token)
              setInspectorTab('dictionary')
            }}
            selectedToken={selectedToken}
            showKana={showKana}
            showPos={showPos}
          />
        </EditorPane>
      </section>

      <InspectorPanel
        activeTab={inspectorTab}
        analysis={analysis}
        analysisSource={analysisSource}
        analysisStatus={analysisState}
        currentProviderId={currentProviderId}
        dictionaryEntry={dictionaryQuery.data ?? null}
        lang={lang}
        analysisCacheStatus={analysisCacheStatus}
        onClearAnalysisCache={() => {
          void handleClearAnalysisCache()
        }}
        onLanguageChange={(value) => updateSetting('lang', value)}
        onPlay={() => {
          void handlePlay()
        }}
        onPlaybackSpeedChange={updatePlaybackSpeed}
        onProviderChange={(value) => updateSetting('ttsProvider', value)}
        onStop={stopPlayback}
        onTabChange={setInspectorTab}
        onThemeChange={(value) => updateSetting('theme', value)}
        onToggleSetting={(key, value) => updateSetting(key, value)}
        providers={providersQuery.data}
        playbackSpeed={playbackSpeed}
        selectedToken={selectedToken}
        showKana={showKana}
        showPos={showPos}
        theme={theme}
      />

      {healthQuery.isLoading ? <div className="status-toast">{t(lang, 'backendConnecting')}</div> : null}
      {healthQuery.isError ? <div className="status-toast status-toast--error">{t(lang, 'backendUnavailable')}</div> : null}
      {documentsQuery.isLoading ? <div className="status-toast">{t(lang, 'loading')}</div> : null}
      {playbackStatus !== 'idle' ? <div className="status-toast floating">TTS active</div> : null}
      {operationMessage ? (
        <div className="status-toast status-toast--error" role="status">
          {operationMessage}
        </div>
      ) : null}

      <div className="quick-actions" aria-label="Quick actions">
        <button
          aria-label="Quick analyze"
          className="quick-actions__button"
          onClick={() => {
            void handleAnalyze()
          }}
          type="button"
        >
          解析
        </button>
        <button
          aria-label={playbackStatus !== 'idle' ? 'Quick stop playback' : 'Quick play document'}
          className="quick-actions__button"
          onClick={() => {
            if (playbackStatus !== 'idle') {
              stopPlayback()
            } else {
              void handlePlay()
            }
          }}
          type="button"
        >
          {playbackStatus !== 'idle' ? '停止' : '播放'}
        </button>
        <button
          aria-label="Quick open dictionary"
          className="quick-actions__button"
          onClick={() => setInspectorTab('dictionary')}
          type="button"
        >
          辞書
        </button>
        <button
          aria-label="Quick open tts"
          className="quick-actions__button quick-actions__button--accent"
          onClick={() => setInspectorTab('tts')}
          type="button"
        >
          TTS
        </button>
      </div>

      {renameDialog ? (
        <div className="modal-backdrop">
          <form
            aria-label={t(lang, 'renameDocument')}
            className="app-dialog"
            onSubmit={(event) => {
              event.preventDefault()
              void confirmRenameDocument()
            }}
            role="dialog"
          >
            <h2>{t(lang, 'renameDocument')}</h2>
            <label className="field">
              <span>{t(lang, 'documentTitle')}</span>
              <input
                autoFocus
                aria-label={t(lang, 'documentTitle')}
                onChange={(event) =>
                  setRenameDialog((current) =>
                    current ? { ...current, title: event.target.value } : current,
                  )
                }
                value={renameDialog.title}
              />
            </label>
            <div className="dialog-actions">
              <button className="secondary-button" onClick={() => setRenameDialog(null)} type="button">
                {t(lang, 'cancel')}
              </button>
              <button className="primary-button" type="submit">
                {t(lang, 'save')}
              </button>
            </div>
          </form>
        </div>
      ) : null}

      {deleteDialog ? (
        <div className="modal-backdrop">
          <section aria-label={t(lang, 'deleteDocument')} className="app-dialog" role="dialog">
            <h2>{t(lang, 'deleteDocument')}</h2>
            <p className="muted">{t(lang, 'deleteDocumentConfirm').replace('{title}', deleteDialog.document.title)}</p>
            <div className="dialog-actions">
              <button className="secondary-button" onClick={() => setDeleteDialog(null)} type="button">
                {t(lang, 'cancel')}
              </button>
              <button className="primary-button danger-button" onClick={() => void confirmDeleteDocument()} type="button">
                {t(lang, 'deleteDocument')}
              </button>
            </div>
          </section>
        </div>
      ) : null}
    </main>
  )
}

type DocumentsLike =
  | {
      documents: DocumentRecord[]
      active_document_id: string | null
    }
  | undefined

function ensureDocumentsShape(current: DocumentsLike) {
  return current ?? { documents: [], active_document_id: null }
}

function updateDocumentsCache(current: DocumentsLike, updated: DocumentRecord) {
  const currentValue = ensureDocumentsShape(current)
  const nextDocuments = currentValue.documents.some((document) => document.id === updated.id)
    ? currentValue.documents.map((document) => (document.id === updated.id ? updated : document))
    : [updated, ...currentValue.documents]

  return {
    documents: nextDocuments.sort((left, right) => right.updated_at - left.updated_at),
    active_document_id: currentValue.active_document_id ?? updated.id,
  }
}

function computeStructureSignature(text: string) {
  if (!text.trim()) {
    return ''
  }
  const lineCount = text.split('\n').filter((line) => line.trim().length > 0).length
  const sentenceBreaks = (text.match(/[。！？!?]/g) ?? []).length
  return `${lineCount}:${sentenceBreaks}`
}

function computeCacheScopeVersion(
  currentProviderId: string,
  providers: { id: string; defaults?: { model: string; voice: string; format: string } }[] | undefined,
) {
  const provider = providers?.find((p) => p.id === currentProviderId)
  const model = provider?.defaults?.model ?? ''
  const voice = provider?.defaults?.voice ?? ''
  const format = provider?.defaults?.format ?? ''
  return `${currentProviderId}:${model}:${voice}:${format}`
}

function normalizePlaybackSpeed(value: unknown) {
  const speed = typeof value === 'number' ? value : Number(value)
  if (!Number.isFinite(speed)) {
    return 1
  }
  return Math.round(Math.min(2, Math.max(0.5, speed)) * 10) / 10
}

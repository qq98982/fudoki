import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, expect, test, vi } from 'vitest'

import App from '../App'

const fetchMock = vi.fn()

beforeEach(() => {
  fetchMock.mockImplementation(async (input: RequestInfo | URL) => {
    const url = String(input)

    if (url.includes('/api/documents')) {
      return new Response(JSON.stringify({ documents: [], active_document_id: null }), {
        headers: { 'Content-Type': 'application/json' },
      })
    }

    if (url.includes('/api/settings')) {
      return new Response(JSON.stringify({ values: { lang: 'en', theme: 'paper' } }), {
        headers: { 'Content-Type': 'application/json' },
      })
    }

    if (url.includes('/api/tts/providers')) {
      return new Response(
        JSON.stringify({
          default_provider: 'system',
          providers: [{ id: 'system', status: 'available' }],
        }),
        {
          headers: { 'Content-Type': 'application/json' },
        },
      )
    }

    return new Response(JSON.stringify({}), {
      headers: { 'Content-Type': 'application/json' },
    })
  })

  vi.stubGlobal('fetch', fetchMock)
})

afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
  cleanup()
})

test('renders the Fudoki workspace shell', async () => {
  render(<App />)

  expect(screen.getByRole('heading', { name: /fudoki workspace/i })).toBeInTheDocument()
  expect(await screen.findByRole('button', { name: /new document/i })).toBeInTheDocument()
  expect(await screen.findByRole('button', { name: /analysis/i })).toBeInTheDocument()
  expect(await screen.findByRole('button', { name: /quick open tts/i })).toBeInTheDocument()
})

test(
  'loads cached analysis, auto-refreshes after idle input, and renders per-line play controls',
  async () => {
  fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)

    if (url.includes('/api/documents/doc-1/analysis?revision=1')) {
      return new Response(
        JSON.stringify({
          lines: [
            [
              {
                surface: '初期',
                lemma: '初期',
                reading: 'ショキ',
                tts_text: 'ショキ',
                pos: ['名詞'],
                source: 'sudachi',
                confidence: 1,
              },
            ],
          ],
        }),
        { headers: { 'Content-Type': 'application/json' } },
      )
    }

    if (url.includes('/api/documents')) {
      return new Response(
        JSON.stringify({
          documents: [
            {
              id: 'doc-1',
              title: 'Test Article',
              title_mode: 'auto',
              content: '初期テキスト',
              source_kind: 'user',
              created_at: 1,
              updated_at: 1,
              revision: 1,
            },
          ],
          active_document_id: 'doc-1',
        }),
        { headers: { 'Content-Type': 'application/json' } },
      )
    }

    if (url.includes('/api/settings')) {
      return new Response(JSON.stringify({ values: { lang: 'en', theme: 'paper' } }), {
        headers: { 'Content-Type': 'application/json' },
      })
    }

    if (url.includes('/api/tts/providers')) {
      return new Response(
        JSON.stringify({
          default_provider: 'system',
          providers: [{ id: 'system', status: 'available' }],
        }),
        { headers: { 'Content-Type': 'application/json' } },
      )
    }

    if (url.endsWith('/api/analyze') && init?.method === 'POST') {
      return new Response(
        JSON.stringify({
          lines: [
            [
              {
                surface: '更新',
                lemma: '更新',
                reading: 'コウシン',
                tts_text: 'コウシン',
                pos: ['名詞'],
                source: 'sudachi',
                confidence: 1,
              },
            ],
          ],
        }),
        { headers: { 'Content-Type': 'application/json' } },
      )
    }

    return new Response(JSON.stringify({}), {
      headers: { 'Content-Type': 'application/json' },
    })
  })

  render(<App />)

  expect((await screen.findAllByText('Initial Analysis')).length).toBeGreaterThan(0)
  expect(await screen.findByText('初期')).toBeInTheDocument()
  expect(await screen.findByRole('button', { name: /play line 1/i })).toBeInTheDocument()

  const editor = (await screen.findAllByRole('textbox', { name: 'Document editor' })).at(-1)!
  fireEvent.change(editor, { target: { value: '更新したテキスト' } })

  // Structure signature hasn't changed (same line count, same sentence breaks),
  // so auto-analysis won't fire. Click the Analyze button to trigger manually.
  const analyzeButtons = screen.getAllByRole('button', { name: /analyze/i })
  fireEvent.click(analyzeButtons[0])

  await waitFor(() => {
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/analyze',
      expect.objectContaining({
        method: 'POST',
      }),
    )
  }, { timeout: 4_000 })

  await waitFor(() => {
    expect(screen.getByText('更新')).toBeInTheDocument()
  }, { timeout: 4_000 })

  expect(await screen.findByRole('button', { name: /play line 1/i })).toBeInTheDocument()
  },
  10_000,
)

test(
  'stop playback still cancels in-flight remote playback after switching documents',
  async () => {
    const deferredSpeech = { resolve: null as null | ((response: Response) => void) }
    const playSpy = vi.fn().mockResolvedValue(undefined)
    const pauseSpy = vi.fn()

    class FakeAudio {
      src = ''
      onended: (() => void) | null = null
      onerror: (() => void) | null = null

      async play() {
        return playSpy()
      }

      pause() {
        pauseSpy()
      }
    }

    vi.stubGlobal('Audio', FakeAudio)
    vi.stubGlobal('URL', {
      createObjectURL: vi.fn(() => 'blob:test-audio'),
      revokeObjectURL: vi.fn(),
    })

    fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input)

      if (url.includes('/api/documents/doc-1/analysis?revision=1')) {
        return new Response(JSON.stringify({ lines: [] }), {
          headers: { 'Content-Type': 'application/json' },
        })
      }

      if (url.endsWith('/api/analyze') && init?.method === 'POST') {
        return new Response(JSON.stringify({ lines: [] }), {
          headers: { 'Content-Type': 'application/json' },
        })
      }

      if (url.endsWith('/api/analyze') && init?.method === 'POST') {
        return new Response(JSON.stringify({ lines: [] }), {
          headers: { 'Content-Type': 'application/json' },
        })
      }

      if (url.includes('/api/documents/doc-2/analysis?revision=1')) {
        return new Response(JSON.stringify({ lines: [] }), {
          headers: { 'Content-Type': 'application/json' },
        })
      }

      if (url.includes('/api/documents')) {
        return new Response(
          JSON.stringify({
            documents: [
              {
                id: 'doc-1',
                title: 'First Article',
                title_mode: 'auto',
                content: '最初の文章',
                source_kind: 'user',
                created_at: 1,
                updated_at: 2,
                revision: 1,
              },
              {
                id: 'doc-2',
                title: 'Second Article',
                title_mode: 'auto',
                content: '次の文章',
                source_kind: 'user',
                created_at: 1,
                updated_at: 1,
                revision: 1,
              },
            ],
            active_document_id: 'doc-1',
          }),
          { headers: { 'Content-Type': 'application/json' } },
        )
      }

      if (url.includes('/api/settings')) {
        return new Response(
          JSON.stringify({ values: { lang: 'en', theme: 'paper', ttsProvider: 'openai-compatible' } }),
          { headers: { 'Content-Type': 'application/json' } },
        )
      }

      if (url.includes('/api/tts/providers')) {
        return new Response(
          JSON.stringify({
            default_provider: 'openai-compatible',
            providers: [
              { id: 'system', status: 'available' },
              {
                id: 'openai-compatible',
                status: 'available',
                defaults: { model: 'gpt-4o-mini-tts', voice: 'alloy', format: 'mp3' },
                options: { models: ['gpt-4o-mini-tts'], voices: ['alloy'] },
              },
            ],
          }),
          { headers: { 'Content-Type': 'application/json' } },
        )
      }

      if (url.endsWith('/api/tts/speak') && init?.method === 'POST') {
        return await new Promise<Response>((resolve) => {
          deferredSpeech.resolve = resolve
        })
      }

      return new Response(JSON.stringify({}), {
        headers: { 'Content-Type': 'application/json' },
      })
    })

    render(<App />)

    await screen.findByText('First Article')
    fireEvent.click(await screen.findByRole('button', { name: /quick open tts/i }))
    fireEvent.click(await screen.findByRole('button', { name: /^Play document$/i }))

    const secondArticleTitle = await screen.findByText('Second Article')
    fireEvent.click(secondArticleTitle.closest('button')!)
    fireEvent.click(await screen.findByRole('button', { name: /stop playback/i }))

    if (deferredSpeech.resolve) {
      deferredSpeech.resolve(
        new Response(new Blob([new Uint8Array([1, 2, 3])], { type: 'audio/mpeg' }), {
          headers: { 'Content-Type': 'audio/mpeg' },
        }),
      )
    }

    await waitFor(() => {
      expect(playSpy).not.toHaveBeenCalled()
    })
  },
  10_000,
)

test(
  'remote full-document playback splits multi-line text into multiple upstream tts requests',
  async () => {
    const playSpy = vi.fn().mockImplementation(function (this: FakeAudio) {
      queueMicrotask(() => this.onended?.())
      return Promise.resolve()
    })
    const pauseSpy = vi.fn()
    const ttsBodies: string[] = []

    class FakeAudio {
      src = ''
      onended: (() => void) | null = null
      onerror: (() => void) | null = null

      async play() {
        return playSpy.call(this)
      }

      pause() {
        pauseSpy()
      }
    }

    vi.stubGlobal('Audio', FakeAudio)
    vi.stubGlobal('URL', {
      createObjectURL: vi.fn(() => 'blob:test-audio'),
      revokeObjectURL: vi.fn(),
    })

    fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input)

      if (url.includes('/api/documents/doc-1/analysis?revision=1')) {
        return new Response(JSON.stringify({ lines: [] }), {
          headers: { 'Content-Type': 'application/json' },
        })
      }

      if (url.endsWith('/api/analyze') && init?.method === 'POST') {
        return new Response(JSON.stringify({ lines: [] }), {
          headers: { 'Content-Type': 'application/json' },
        })
      }

      if (url.includes('/api/documents')) {
        return new Response(
          JSON.stringify({
            documents: [
              {
                id: 'doc-1',
                title: 'First Article',
                title_mode: 'auto',
                content: '第一文です。\n第二文です。',
                source_kind: 'user',
                created_at: 1,
                updated_at: 2,
                revision: 1,
              },
            ],
            active_document_id: 'doc-1',
          }),
          { headers: { 'Content-Type': 'application/json' } },
        )
      }

      if (url.includes('/api/settings')) {
        return new Response(
          JSON.stringify({ values: { lang: 'en', theme: 'paper', ttsProvider: 'openai-compatible' } }),
          { headers: { 'Content-Type': 'application/json' } },
        )
      }

      if (url.includes('/api/tts/providers')) {
        return new Response(
          JSON.stringify({
            default_provider: 'openai-compatible',
            providers: [
              { id: 'system', status: 'available' },
              {
                id: 'openai-compatible',
                status: 'available',
                defaults: { model: 'gpt-4o-mini-tts', voice: 'alloy', format: 'mp3' },
                options: { models: ['gpt-4o-mini-tts'], voices: ['alloy'] },
              },
            ],
          }),
          { headers: { 'Content-Type': 'application/json' } },
        )
      }

      if (url.endsWith('/api/tts/speak') && init?.method === 'POST') {
        ttsBodies.push(String(init.body))
        return new Response(new Blob([new Uint8Array([1, 2, 3])], { type: 'audio/mpeg' }), {
          headers: { 'Content-Type': 'audio/mpeg' },
        })
      }

      return new Response(JSON.stringify({}), {
        headers: { 'Content-Type': 'application/json' },
      })
    })

    render(<App />)

    await screen.findByText('First Article')
    fireEvent.click(await screen.findByRole('button', { name: /quick open tts/i }))
    fireEvent.click(await screen.findByRole('button', { name: /^Play document$/i }))

    await waitFor(() => {
      expect(ttsBodies.length).toBeGreaterThan(1)
    })

    expect(ttsBodies).toHaveLength(2)
    expect(ttsBodies[0]).toContain('第一文です。')
    expect(ttsBodies[1]).toContain('第二文です。')
  },
  10_000,
)

test(
  'remote segmented playback stops without requesting later segments',
  async () => {
    const ttsBodies: string[] = []
    const audioInstances: FakeAudio[] = []

    class FakeAudio {
      src = ''
      onended: (() => void) | null = null
      onerror: (() => void) | null = null
      constructor() {
        audioInstances.push(this)
      }

      async play() {
        return Promise.resolve()
      }

      pause() {}
    }

    vi.stubGlobal('Audio', FakeAudio)
    vi.stubGlobal('URL', {
      createObjectURL: vi.fn(() => 'blob:test-audio'),
      revokeObjectURL: vi.fn(),
    })

    fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input)

      if (url.includes('/api/documents/doc-1/analysis?revision=1')) {
        return new Response(JSON.stringify({ lines: [] }), {
          headers: { 'Content-Type': 'application/json' },
        })
      }

      if (url.includes('/api/documents')) {
        return new Response(
          JSON.stringify({
            documents: [
              {
                id: 'doc-1',
                title: 'First Article',
                title_mode: 'auto',
                content: '第一文です。\n第二文です。',
                source_kind: 'user',
                created_at: 1,
                updated_at: 2,
                revision: 1,
              },
            ],
            active_document_id: 'doc-1',
          }),
          { headers: { 'Content-Type': 'application/json' } },
        )
      }

      if (url.includes('/api/settings')) {
        return new Response(
          JSON.stringify({ values: { lang: 'en', theme: 'paper', ttsProvider: 'openai-compatible' } }),
          { headers: { 'Content-Type': 'application/json' } },
        )
      }

      if (url.includes('/api/tts/providers')) {
        return new Response(
          JSON.stringify({
            default_provider: 'openai-compatible',
            providers: [
              { id: 'system', status: 'available' },
              {
                id: 'openai-compatible',
                status: 'available',
                defaults: { model: 'gpt-4o-mini-tts', voice: 'alloy', format: 'mp3' },
                options: { models: ['gpt-4o-mini-tts'], voices: ['alloy'] },
              },
            ],
          }),
          { headers: { 'Content-Type': 'application/json' } },
        )
      }

      if (url.endsWith('/api/tts/speak') && init?.method === 'POST') {
        ttsBodies.push(String(init.body))
        return new Response(new Blob([new Uint8Array([1, 2, 3])], { type: 'audio/mpeg' }), {
          headers: { 'Content-Type': 'audio/mpeg' },
        })
      }

      return new Response(JSON.stringify({}), {
        headers: { 'Content-Type': 'application/json' },
      })
    })

    render(<App />)

    await screen.findByText('First Article')
    fireEvent.click(await screen.findByRole('button', { name: /quick open tts/i }))
    fireEvent.click(await screen.findByRole('button', { name: /^Play document$/i }))

    await waitFor(() => {
      expect(ttsBodies).toHaveLength(1)
      expect(audioInstances).toHaveLength(1)
    })

    fireEvent.click(await screen.findByRole('button', { name: /^Stop playback$/i }))

    await waitFor(() => {
      expect(ttsBodies).toHaveLength(1)
    })

    expect(audioInstances[0].onended).toBeTruthy()
  },
  10_000,
)

test('renames documents through an in-app dialog', async () => {
  const promptMock = vi.fn()
  vi.stubGlobal('prompt', promptMock)

  fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)

    if (url.includes('/api/documents/doc-1') && init?.method === 'PUT') {
      return new Response(
        JSON.stringify({
          document: {
            id: 'doc-1',
            title: 'Renamed Article',
            title_mode: 'custom',
            content: '本文',
            source_kind: 'user',
            created_at: 1,
            updated_at: 3,
            revision: 2,
          },
        }),
        { headers: { 'Content-Type': 'application/json' } },
      )
    }

    if (url.includes('/api/documents/doc-1/analysis?revision=1')) {
      return new Response(JSON.stringify({ lines: [] }), {
        headers: { 'Content-Type': 'application/json' },
      })
    }

    if (url.includes('/api/documents')) {
      return new Response(
        JSON.stringify({
          documents: [
            {
              id: 'doc-1',
              title: 'First Article',
              title_mode: 'custom',
              content: '本文',
              source_kind: 'user',
              created_at: 1,
              updated_at: 2,
              revision: 1,
            },
          ],
          active_document_id: 'doc-1',
        }),
        { headers: { 'Content-Type': 'application/json' } },
      )
    }

    if (url.includes('/api/settings')) {
      return new Response(JSON.stringify({ values: { lang: 'en', theme: 'paper' } }), {
        headers: { 'Content-Type': 'application/json' },
      })
    }

    if (url.includes('/api/tts/providers')) {
      return new Response(
        JSON.stringify({
          default_provider: 'system',
          providers: [{ id: 'system', status: 'available' }],
        }),
        { headers: { 'Content-Type': 'application/json' } },
      )
    }

    return new Response(JSON.stringify({}), {
      headers: { 'Content-Type': 'application/json' },
    })
  })

  render(<App />)

  await screen.findByText('First Article')
  fireEvent.click(await screen.findByRole('button', { name: /rename/i }))

  expect(promptMock).not.toHaveBeenCalled()
  expect(await screen.findByRole('dialog', { name: /rename document/i })).toBeInTheDocument()

  const titleInput = await screen.findByRole('textbox', { name: /document title/i })
  fireEvent.change(titleInput, { target: { value: 'Renamed Article' } })
  fireEvent.click(await screen.findByRole('button', { name: /^save$/i }))

  await waitFor(() => {
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/documents/doc-1',
      expect.objectContaining({
        method: 'PUT',
        body: expect.stringContaining('Renamed Article'),
      }),
    )
  })
  expect(await screen.findByText('Renamed Article')).toBeInTheDocument()
})

test('confirms document deletion through an in-app dialog', async () => {
  const confirmMock = vi.fn()
  vi.stubGlobal('confirm', confirmMock)

  fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)

    if (url.includes('/api/documents/doc-1') && init?.method === 'DELETE') {
      return new Response(null, { status: 204 })
    }

    if (url.includes('/api/documents/doc-1/analysis?revision=1')) {
      return new Response(JSON.stringify({ lines: [] }), {
        headers: { 'Content-Type': 'application/json' },
      })
    }

    if (url.includes('/api/documents')) {
      return new Response(
        JSON.stringify({
          documents: [
            {
              id: 'doc-1',
              title: 'First Article',
              title_mode: 'auto',
              content: '本文',
              source_kind: 'user',
              created_at: 1,
              updated_at: 2,
              revision: 1,
            },
          ],
          active_document_id: 'doc-1',
        }),
        { headers: { 'Content-Type': 'application/json' } },
      )
    }

    if (url.includes('/api/settings')) {
      return new Response(JSON.stringify({ values: { lang: 'en', theme: 'paper' } }), {
        headers: { 'Content-Type': 'application/json' },
      })
    }

    if (url.includes('/api/tts/providers')) {
      return new Response(
        JSON.stringify({
          default_provider: 'system',
          providers: [{ id: 'system', status: 'available' }],
        }),
        { headers: { 'Content-Type': 'application/json' } },
      )
    }

    return new Response(JSON.stringify({}), {
      headers: { 'Content-Type': 'application/json' },
    })
  })

  render(<App />)

  await screen.findByText('First Article')
  fireEvent.click(await screen.findByRole('button', { name: /^delete$/i }))

  expect(confirmMock).not.toHaveBeenCalled()
  expect(await screen.findByRole('dialog', { name: /delete document/i })).toBeInTheDocument()

  fireEvent.click(await screen.findByRole('button', { name: /^delete document$/i }))

  await waitFor(() => {
    expect(fetchMock).toHaveBeenCalledWith('/api/documents/doc-1', { method: 'DELETE' })
  })
})

test('shows operation feedback when duplicate fails', async () => {
  fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)

    if (url.includes('/api/documents/doc-1/duplicate') && init?.method === 'POST') {
      return new Response(JSON.stringify({ error: { code: 'internal_error', message: 'duplicate failed' } }), {
        headers: { 'Content-Type': 'application/json' },
        status: 500,
      })
    }

    if (url.includes('/api/documents/doc-1/analysis?revision=1')) {
      return new Response(JSON.stringify({ lines: [] }), {
        headers: { 'Content-Type': 'application/json' },
      })
    }

    if (url.includes('/api/documents')) {
      return new Response(
        JSON.stringify({
          documents: [
            {
              id: 'doc-1',
              title: 'First Article',
              title_mode: 'auto',
              content: '本文',
              source_kind: 'user',
              created_at: 1,
              updated_at: 2,
              revision: 1,
            },
          ],
          active_document_id: 'doc-1',
        }),
        { headers: { 'Content-Type': 'application/json' } },
      )
    }

    if (url.includes('/api/settings')) {
      return new Response(JSON.stringify({ values: { lang: 'en', theme: 'paper' } }), {
        headers: { 'Content-Type': 'application/json' },
      })
    }

    if (url.includes('/api/tts/providers')) {
      return new Response(
        JSON.stringify({
          default_provider: 'system',
          providers: [{ id: 'system', status: 'available' }],
        }),
        { headers: { 'Content-Type': 'application/json' } },
      )
    }

    return new Response(JSON.stringify({}), {
      headers: { 'Content-Type': 'application/json' },
    })
  })

  render(<App />)

  await screen.findByText('First Article')
  fireEvent.click(await screen.findByRole('button', { name: /duplicate/i }))

  expect(await screen.findByRole('status')).toHaveTextContent(/duplicate failed/i)
})

test('settings panel clears analysis cache after confirmation and shows success feedback', async () => {
  const confirmMock = vi.fn(() => true)
  vi.stubGlobal('confirm', confirmMock)

  fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)

    if (url.includes('/api/documents')) {
      return new Response(JSON.stringify({ documents: [], active_document_id: null }), {
        headers: { 'Content-Type': 'application/json' },
      })
    }

    if (url.includes('/api/settings')) {
      return new Response(JSON.stringify({ values: { lang: 'en', theme: 'paper' } }), {
        headers: { 'Content-Type': 'application/json' },
      })
    }

    if (url.includes('/api/tts/providers')) {
      return new Response(
        JSON.stringify({
          default_provider: 'system',
          providers: [{ id: 'system', status: 'available' }],
        }),
        {
          headers: { 'Content-Type': 'application/json' },
        },
      )
    }

    if (url.endsWith('/api/analysis-cache') && init?.method === 'DELETE') {
      return new Response(null, { status: 204 })
    }

    return new Response(JSON.stringify({}), {
      headers: { 'Content-Type': 'application/json' },
    })
  })

  render(<App />)

  fireEvent.click(await screen.findByRole('button', { name: /settings/i }))
  fireEvent.click(await screen.findByRole('button', { name: /clear analysis cache/i }))

  expect(confirmMock).toHaveBeenCalled()
  await waitFor(() => {
    expect(fetchMock).toHaveBeenCalledWith('/api/analysis-cache', { method: 'DELETE' })
  })
  expect(await screen.findByText('Analysis cache cleared.')).toBeInTheDocument()
})

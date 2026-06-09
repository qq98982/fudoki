import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, test } from 'vitest'

const repoRoot = resolve(process.cwd(), '..')

function readRepoFile(path: string) {
  return readFileSync(resolve(repoRoot, path), 'utf8')
}

function stripHtmlComments(source: string) {
  return source.replace(/<!--[\s\S]*?-->/g, '')
}

describe('source guards', () => {
  test('legacy static page has unique element ids', () => {
    const html = stripHtmlComments(readRepoFile('index.html'))
    const ids = Array.from(html.matchAll(/\bid="([^"]+)"/g), (match) => match[1])
    const duplicates = ids.filter((id, index) => ids.indexOf(id) !== index)

    expect(duplicates).toEqual([])
  })

  test('backend root route does not silently fall back to the legacy static page', () => {
    const appSource = readRepoFile('src/app.rs')

    expect(appSource).not.toContain('PathBuf::from("index.html")')
    expect(appSource).not.toContain('fallback_path')
  })

  test('mobile layout prioritizes the editor and keeps quick actions in normal flow', () => {
    const css = readRepoFile('frontend/src/index.css')

    expect(css).toContain('.workspace-center {\n    order: 1;')
    expect(css).toContain('.document-rail {\n    order: 2;')
    expect(css).toContain('.quick-actions {\n    position: sticky;')
    expect(css).toContain('.editor-input {\n    min-height: 280px;')
  })
})

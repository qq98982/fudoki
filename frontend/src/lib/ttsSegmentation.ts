export interface TtsSegment {
  text: string
  pause: number
}

export function normalizeTextForRemoteTts(text: string) {
  return String(text || '')
    .replace(/\r\n/g, '\n')
    .replace(/（[^）]*）|\([^)]*\)/g, '')
    .replace(/[、，]/g, ' ')
    .replace(/[^\S\n\r\u00A0]+/g, ' ')
    .trim()
}

export function splitTextByPunctuation(text: string): TtsSegment[] {
  const normalized = normalizeTextForRemoteTts(text)
  const segments: TtsSegment[] = []
  const heavyPause = 0
  const mediumPause = 0
  const lightPause = 0
  const ellipsisPause = 0
  const linePause = 120
  const titleLinePause = 180
  const paragraphPause = 260

  let buffer = ''

  const isTitleLikeLine = (value: string) => {
    const trimmed = String(value || '').trim()
    if (!trimmed) return false
    if (trimmed.length > 16) return false
    if (/[。！？!?：:；;]$/.test(trimmed)) return false
    if (/^[#>\-\s]+$/.test(trimmed)) return false
    return true
  }

  const pushSegment = (pause: number) => {
    const segmentText = buffer.trim()
    if (segmentText) {
      segments.push({ text: segmentText, pause })
    }
    buffer = ''
  }

  for (let index = 0; index < normalized.length; index += 1) {
    const char = normalized[index]
    const next = normalized[index + 1] || ''
    const next2 = normalized[index + 2] || ''

    if (char === '\n') {
      const currentLine = buffer.trim()
      let newlineCount = 1
      while (normalized[index + newlineCount] === '\n') {
        newlineCount += 1
      }
      const remaining = normalized.slice(index + newlineCount).trim()
      let pause = linePause
      if (newlineCount > 1) {
        pause = paragraphPause
      } else if (remaining && isTitleLikeLine(currentLine)) {
        pause = titleLinePause
      }
      pushSegment(pause)
      index += newlineCount - 1
      continue
    }

    buffer += char

    if (char === '…') {
      while (normalized[index + 1] === '…') {
        buffer += normalized[++index]
      }
      pushSegment(ellipsisPause)
      continue
    }

    if (char === '.' && next === '.' && next2 === '.') {
      buffer += next + next2
      index += 2
      pushSegment(ellipsisPause)
      continue
    }

    if ('。！？!?？！'.includes(char)) {
      pushSegment(heavyPause)
      continue
    }

    if ('、，,；;'.includes(char)) {
      pushSegment(mediumPause)
      continue
    }

    if ('：:'.includes(char)) {
      pushSegment(lightPause)
    }
  }

  pushSegment(0)
  return segments
}

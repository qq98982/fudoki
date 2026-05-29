interface SystemSpeechCallbacks {
  onStart?: () => void
  onEnd?: () => void
  onError?: () => void
}

export function stopSystemSpeech() {
  if (typeof window === 'undefined' || !window.speechSynthesis) {
    return
  }
  window.speechSynthesis.cancel()
}

export function speakWithSystemSpeech(text: string, rate = 1, callbacks: SystemSpeechCallbacks = {}) {
  if (typeof window === 'undefined' || !window.speechSynthesis) {
    throw new Error('Speech synthesis is not available in this browser')
  }

  stopSystemSpeech()

  const utterance = new SpeechSynthesisUtterance(text)
  utterance.rate = Math.max(0.5, Math.min(2, rate))
  utterance.lang = 'ja-JP'
  utterance.onstart = () => callbacks.onStart?.()
  utterance.onend = () => callbacks.onEnd?.()
  utterance.onerror = () => callbacks.onError?.()
  window.speechSynthesis.speak(utterance)
  return utterance
}

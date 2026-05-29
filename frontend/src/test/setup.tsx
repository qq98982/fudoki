import '@testing-library/jest-dom/vitest'
import { vi } from 'vitest'

// jsdom doesn't support the DOM APIs CodeMirror 6 needs (e.g. getClientRects).
// Replace it with a plain textarea in tests so existing test assertions work.
vi.mock('@uiw/react-codemirror', () => {
  return {
    default: (props: Record<string, unknown>) => {
      const { value, onChange, placeholder, className } = props
      const handleChange = (e: { target: { value: string } }) => {
        if (typeof onChange === 'function') {
          onChange(e.target.value)
        }
      }
      // Return a plain textarea that Testing Library will find
      return (
        <textarea
          aria-label="Document editor"
          className={typeof className === 'string' ? className : ''}
          placeholder={typeof placeholder === 'string' ? placeholder : ''}
          value={typeof value === 'string' ? value : ''}
          onChange={handleChange}
        />
      )
    },
  }
})

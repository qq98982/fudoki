import '@testing-library/jest-dom/vitest'
import { vi } from 'vitest'

// jsdom doesn't support the DOM APIs CodeMirror 6 needs (e.g. getClientRects).
// Replace it with a plain textarea in tests so existing test assertions work.
vi.mock('@uiw/react-codemirror', () => {
  const React = require('react')
  return {
    default: React.forwardRef(
      (
        props: {
          value?: string
          onChange?: (value: string) => void
          placeholder?: string
          'aria-label'?: string
          className?: string
        },
        ref: React.Ref<HTMLTextAreaElement>,
      ) =>
        React.createElement('textarea', {
          ref,
          'aria-label': props['aria-label'],
          className: props.className,
          placeholder: props.placeholder,
          value: props.value ?? '',
          onChange: (e: React.ChangeEvent<HTMLTextAreaElement>) => {
            props.onChange?.(e.target.value)
          },
        }),
    ),
  }
})

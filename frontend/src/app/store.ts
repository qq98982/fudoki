import { create } from 'zustand'

import type { AnalyzeToken, InspectorTab } from '../types'

interface AppStoreState {
  activeDocumentId: string | null
  inspectorTab: InspectorTab
  selectedToken: AnalyzeToken | null
  search: string
  setActiveDocumentId: (id: string | null) => void
  setInspectorTab: (tab: InspectorTab) => void
  setSelectedToken: (token: AnalyzeToken | null) => void
  setSearch: (value: string) => void
}

export const useAppStore = create<AppStoreState>((set) => ({
  activeDocumentId: null,
  inspectorTab: 'analysis',
  selectedToken: null,
  search: '',
  setActiveDocumentId: (activeDocumentId) => set({ activeDocumentId }),
  setInspectorTab: (inspectorTab) => set({ inspectorTab }),
  setSelectedToken: (selectedToken) => set({ selectedToken }),
  setSearch: (search) => set({ search }),
}))

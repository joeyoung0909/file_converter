import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export type AppStatus = 'idle' | 'scanning' | 'preview' | 'converting' | 'done'

export type IdentifyStatus = 'Identified' | 'MultipleCandidates' | 'NotFound' | 'Unconfirmed'

export interface FileInfo {
  path: string
  name: string
  size: number
  modified: number
  isLarge: boolean
}

export interface IdentifyResult {
  status: IdentifyStatus
  candidates: FileInfo[]
  selectedIndex: number | null
}

export interface StudentEntry {
  studentName: string
  studentPath: string
  identifyResult: IdentifyResult
}

export interface ConvertTask {
  studentName: string
  sourcePath: string
  outputDir: string
}

export interface ConvertResult {
  studentName: string
  sourcePath: string
  outputPath: string | null
  success: boolean
  error: string | null
  skipped: boolean
  skipReason: string | null
}

export interface ConversionReport {
  total: number
  succeeded: number
  failed: number
  skipped: number
  results: ConvertResult[]
}

export interface ConversionProgress {
  current: number
  total: number
  currentFile: string
  status: string
}

export interface OutputConfig {
  mode: 'same' | 'custom'
  customDir: string | null
  existingPdfStrategy: 'overwrite' | 'skip' | 'rename'
}

export interface Settings {
  includeKeywords: string[]
  excludeKeywords: string[]
  searchDepth: number
  concurrency: number
  outputConfig: OutputConfig
}

// Which student entries are selected for conversion (by index)
export interface EntrySelection {
  checked: boolean
  // Override selected file index (for MultipleCandidates / user manual override)
  selectedFileIndex: number | null
  // User-manually specified path (for NotFound case)
  manualPath: string | null
}

const DEFAULT_SETTINGS: Settings = {
  includeKeywords: [
    '毕业论文', '毕业设计', '学位论文', '毕设', '论文终稿', '论文定稿', '最终版',
    'thesis', 'dissertation', 'final_paper',
  ],
  excludeKeywords: [
    '开题报告', '任务书', '文献综述', '文献翻译', '外文翻译', '中期报告',
    '答辩', 'ppt', '幻灯片', '成绩',
  ],
  searchDepth: 2,
  concurrency: 2,
  outputConfig: {
    mode: 'same',
    customDir: null,
    existingPdfStrategy: 'skip',
  },
}

export const useConversionStore = defineStore('conversion', () => {
  const status = ref<AppStatus>('idle')
  const rootFolder = ref<string>('')
  const entries = ref<StudentEntry[]>([])
  const selections = ref<EntrySelection[]>([])
  const progress = ref<ConversionProgress | null>(null)
  const report = ref<ConversionReport | null>(null)
  const libreOfficePath = ref<string | null>(null)
  const settings = ref<Settings>({ ...DEFAULT_SETTINGS })
  const errorMessage = ref<string | null>(null)

  let unlistenProgress: UnlistenFn | null = null

  const selectedCount = computed(() =>
    selections.value.filter((s) => s.checked).length
  )

  async function checkLibreOffice() {
    try {
      libreOfficePath.value = await invoke<string | null>('get_libreoffice_path')
    } catch {
      libreOfficePath.value = null
    }
  }

  async function loadDefaultConcurrency() {
    try {
      settings.value.concurrency = await invoke<number>('get_default_concurrency')
    } catch {
      settings.value.concurrency = 2
    }
  }

  async function scanFolder(folderPath: string) {
    status.value = 'scanning'
    errorMessage.value = null
    rootFolder.value = folderPath

    try {
      const result = await invoke<StudentEntry[]>('scan_folder', {
        path: folderPath,
        depth: settings.value.searchDepth,
        keywordConfig: {
          includeKeywords: settings.value.includeKeywords,
          excludeKeywords: settings.value.excludeKeywords,
        },
      })

      entries.value = result
      selections.value = result.map((entry) => ({
        checked: entry.identifyResult.status !== 'NotFound',
        selectedFileIndex: entry.identifyResult.selectedIndex,
        manualPath: null,
      }))
      status.value = 'preview'
    } catch (e) {
      errorMessage.value = String(e)
      status.value = 'idle'
    }
  }

  function selectAll() {
    selections.value.forEach((s) => (s.checked = true))
  }

  function deselectAll() {
    selections.value.forEach((s) => (s.checked = false))
  }

  async function startConversion() {
    errorMessage.value = null
    progress.value = null
    report.value = null

    const tasks: ConvertTask[] = entries.value
      .map((entry, i) => {
        const sel = selections.value[i]
        if (!sel.checked) return null

        let sourcePath: string | null = null
        if (sel.manualPath) {
          sourcePath = sel.manualPath
        } else if (sel.selectedFileIndex !== null) {
          const candidate = entry.identifyResult.candidates[sel.selectedFileIndex]
          sourcePath = candidate?.path ?? null
        } else if (entry.identifyResult.candidates.length > 0) {
          sourcePath = entry.identifyResult.candidates[0].path
        }

        if (!sourcePath) return null

        return {
          studentName: entry.studentName,
          sourcePath,
          outputDir: '', // resolved in Rust
        } as ConvertTask
      })
      .filter((t): t is ConvertTask => t !== null)

    if (tasks.length === 0) {
      errorMessage.value = '没有选中任何文件'
      return
    }

    status.value = 'converting'

    // Subscribe to progress events
    unlistenProgress = await listen<ConversionProgress>('conversion-progress', (event) => {
      progress.value = event.payload
    })

    try {
      const result = await invoke<ConversionReport>('start_conversion', {
        request: {
          tasks,
          outputConfig: settings.value.outputConfig,
          concurrency: settings.value.concurrency,
        },
      })
      report.value = result
    } catch (e) {
      errorMessage.value = String(e)
    } finally {
      unlistenProgress?.()
      unlistenProgress = null
      status.value = 'done'
    }
  }

  async function cancelConversion() {
    await invoke('cancel_conversion')
  }

  async function exportReport(format: 'txt' | 'csv', filePath: string) {
    if (!report.value) return
    await invoke('export_report', {
      report: report.value,
      format,
      path: filePath,
    })
  }

  function reset() {
    status.value = 'idle'
    rootFolder.value = ''
    entries.value = []
    selections.value = []
    progress.value = null
    report.value = null
    errorMessage.value = null
  }

  return {
    status,
    rootFolder,
    entries,
    selections,
    progress,
    report,
    libreOfficePath,
    settings,
    errorMessage,
    selectedCount,
    checkLibreOffice,
    loadDefaultConcurrency,
    scanFolder,
    selectAll,
    deselectAll,
    startConversion,
    cancelConversion,
    exportReport,
    reset,
  }
})

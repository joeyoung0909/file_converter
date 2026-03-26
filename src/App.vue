<template>
  <div class="min-h-screen flex flex-col">
    <!-- Header -->
    <header class="bg-white border-b border-gray-200 px-6 py-3 flex items-center justify-between shadow-sm">
      <h1 class="text-base font-semibold text-gray-800">毕业论文 Word → PDF 批量转换</h1>
      <div class="flex items-center gap-3">
        <!-- LibreOffice status indicator -->
        <div class="flex items-center gap-1.5 text-xs">
          <span
            class="inline-block w-2 h-2 rounded-full"
            :class="store.libreOfficePath ? 'bg-green-500' : 'bg-red-400'"
          />
          <span class="text-gray-500">
            {{ store.libreOfficePath ? 'LibreOffice 就绪' : 'LibreOffice 未找到' }}
          </span>
        </div>

        <button
          class="p-2 rounded-lg hover:bg-gray-100 transition-colors text-gray-500"
          title="设置"
          @click="showSettings = true"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
          </svg>
        </button>
      </div>
    </header>

    <!-- Main content -->
    <main class="flex-1 p-6 overflow-auto">

      <!-- Error banner -->
      <div
        v-if="store.errorMessage"
        class="mb-4 p-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-700 flex items-start gap-2"
      >
        <span class="shrink-0">⚠</span>
        <span>{{ store.errorMessage }}</span>
        <button class="ml-auto text-red-400 hover:text-red-600" @click="store.errorMessage = null">×</button>
      </div>

      <!-- LibreOffice missing warning -->
      <div
        v-if="!store.libreOfficePath && store.status !== 'converting'"
        class="mb-4 p-3 bg-amber-50 border border-amber-200 rounded-xl text-sm text-amber-700"
      >
        未找到 LibreOffice。请先安装 LibreOffice（<span class="font-mono">https://www.libreoffice.org/download/</span>），然后重启本工具。
      </div>

      <!-- IDLE: drop zone -->
      <div v-if="store.status === 'idle'" class="max-w-2xl mx-auto mt-12">
        <DropZone @select="onFolderSelect" />
      </div>

      <!-- SCANNING: loading -->
      <div v-else-if="store.status === 'scanning'" class="flex flex-col items-center justify-center mt-24 gap-4">
        <div class="w-10 h-10 border-4 border-blue-500 border-t-transparent rounded-full animate-spin" />
        <p class="text-gray-500">正在扫描文件夹...</p>
      </div>

      <!-- PREVIEW: results table + start button -->
      <div v-else-if="store.status === 'preview'" class="flex flex-col gap-4">
        <div class="flex items-center gap-3">
          <button
            class="text-sm text-gray-400 hover:text-gray-600"
            @click="store.reset()"
          >← 重新选择</button>
          <span class="text-sm text-gray-500 truncate">{{ store.rootFolder }}</span>
        </div>

        <ResultTable />

        <!-- Output config row -->
        <div class="bg-white border border-gray-200 rounded-xl p-4 flex items-center gap-4 flex-wrap">
          <span class="text-sm font-medium text-gray-700">输出位置：</span>
          <label class="flex items-center gap-2 text-sm">
            <input type="radio" v-model="store.settings.outputConfig.mode" value="same" />
            原文件目录
          </label>
          <label class="flex items-center gap-2 text-sm">
            <input type="radio" v-model="store.settings.outputConfig.mode" value="custom" />
            指定目录
          </label>
          <template v-if="store.settings.outputConfig.mode === 'custom'">
            <button
              class="px-3 py-1.5 text-sm border rounded-lg hover:bg-gray-50 transition-colors"
              @click="selectOutputDir"
            >浏览...</button>
            <span
              v-if="store.settings.outputConfig.customDir"
              class="text-xs text-gray-500 truncate max-w-xs"
              :title="store.settings.outputConfig.customDir"
            >
              {{ store.settings.outputConfig.customDir }}
            </span>
            <span v-else class="text-xs text-red-400">请先选择输出目录</span>
          </template>
        </div>

        <!-- Start button -->
        <div class="flex justify-end">
          <button
            class="px-6 py-2.5 bg-blue-500 text-white rounded-xl font-medium hover:bg-blue-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            :disabled="store.selectedCount === 0 || !store.libreOfficePath || (store.settings.outputConfig.mode === 'custom' && !store.settings.outputConfig.customDir)"
            @click="store.startConversion()"
          >
            开始转换 ({{ store.selectedCount }}/{{ store.entries.length }})
          </button>
        </div>
      </div>

      <!-- CONVERTING: progress -->
      <div v-else-if="store.status === 'converting'" class="max-w-2xl mx-auto mt-16 flex flex-col gap-4">
        <ProgressBar v-if="store.progress" :progress="store.progress" />
        <div v-else class="flex flex-col items-center gap-4">
          <div class="w-10 h-10 border-4 border-blue-500 border-t-transparent rounded-full animate-spin" />
          <p class="text-gray-500">准备中...</p>
        </div>
      </div>

      <!-- DONE: report -->
      <div v-else-if="store.status === 'done'" class="max-w-2xl mx-auto mt-8">
        <ResultReport v-if="store.report" :report="store.report" />
      </div>

    </main>

    <!-- Settings modal -->
    <SettingsModal v-if="showSettings" @close="showSettings = false" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { useConversionStore } from '@/stores/conversion'
import DropZone from '@/components/DropZone.vue'
import ResultTable from '@/components/ResultTable.vue'
import ProgressBar from '@/components/ProgressBar.vue'
import ResultReport from '@/components/ResultReport.vue'
import SettingsModal from '@/components/SettingsModal.vue'

const store = useConversionStore()
const showSettings = ref(false)

onMounted(async () => {
  await store.checkLibreOffice()
  await store.loadDefaultConcurrency()
})

async function onFolderSelect(path: string) {
  await store.scanFolder(path)
}

async function selectOutputDir() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: '选择 PDF 输出目录',
  })
  if (typeof selected === 'string') {
    store.settings.outputConfig.customDir = selected
  }
}
</script>

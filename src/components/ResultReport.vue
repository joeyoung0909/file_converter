<template>
  <div class="bg-white border border-gray-200 rounded-xl p-6 shadow-sm">
    <!-- Summary -->
    <h3 class="text-lg font-semibold text-gray-800 mb-4">转换完成</h3>
    <div class="grid grid-cols-3 gap-4 mb-6">
      <div class="text-center p-3 bg-green-50 rounded-xl">
        <div class="text-2xl font-bold text-green-600">{{ report.succeeded }}</div>
        <div class="text-xs text-green-700 mt-1">成功</div>
      </div>
      <div class="text-center p-3 bg-red-50 rounded-xl">
        <div class="text-2xl font-bold text-red-600">{{ report.failed }}</div>
        <div class="text-xs text-red-700 mt-1">失败</div>
      </div>
      <div class="text-center p-3 bg-gray-50 rounded-xl">
        <div class="text-2xl font-bold text-gray-500">{{ report.skipped }}</div>
        <div class="text-xs text-gray-600 mt-1">跳过</div>
      </div>
    </div>

    <!-- Failed / Skipped details -->
    <div v-if="problemItems.length > 0" class="mb-5">
      <h4 class="text-sm font-medium text-gray-600 mb-2">问题详情</h4>
      <div class="max-h-48 overflow-y-auto border border-gray-100 rounded-lg divide-y divide-gray-100">
        <div
          v-for="item in problemItems"
          :key="item.sourcePath"
          class="px-3 py-2 text-sm"
        >
          <div class="flex items-start gap-2">
            <span :class="item.skipped ? 'text-gray-400' : 'text-red-500'">
              {{ item.skipped ? '—' : '✗' }}
            </span>
            <div>
              <div class="font-medium text-gray-700">{{ item.studentName }}</div>
              <div class="text-gray-400 text-xs truncate">{{ item.sourcePath }}</div>
              <div v-if="item.error || item.skipReason" class="text-xs mt-0.5"
                :class="item.skipped ? 'text-gray-500' : 'text-red-500'"
              >
                {{ item.error || item.skipReason }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Actions -->
    <div class="flex items-center gap-3">
      <button
        class="px-4 py-2 text-sm border rounded-lg hover:bg-gray-50 transition-colors"
        @click="exportAs('txt')"
      >导出报告 (.txt)</button>
      <button
        class="px-4 py-2 text-sm border rounded-lg hover:bg-gray-50 transition-colors"
        @click="exportAs('csv')"
      >导出报告 (.csv)</button>
      <button
        class="ml-auto px-5 py-2 text-sm bg-blue-500 text-white rounded-lg hover:bg-blue-600 transition-colors"
        @click="store.reset()"
      >重新开始</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { save } from '@tauri-apps/plugin-dialog'
import { useConversionStore, type ConversionReport } from '@/stores/conversion'

const props = defineProps<{ report: ConversionReport }>()
const store = useConversionStore()

const problemItems = computed(() =>
  props.report.results.filter((r) => !r.success)
)

async function exportAs(format: 'txt' | 'csv') {
  const ext = format === 'csv' ? 'csv' : 'txt'
  const path = await save({
    defaultPath: `转换报告.${ext}`,
    filters: [{ name: format.toUpperCase(), extensions: [ext] }],
  })
  if (path) {
    await store.exportReport(format, path)
  }
}
</script>

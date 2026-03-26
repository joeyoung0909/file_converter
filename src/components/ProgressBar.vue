<template>
  <div class="bg-white border border-gray-200 rounded-xl p-5 shadow-sm">
    <div class="flex items-center justify-between mb-3">
      <span class="font-medium text-gray-700">
        正在转换 ({{ progress.current }}/{{ progress.total }})
      </span>
      <button
        class="px-3 py-1 text-sm text-red-600 border border-red-300 rounded-lg hover:bg-red-50 transition-colors"
        @click="store.cancelConversion()"
      >
        取消
      </button>
    </div>

    <!-- Progress bar -->
    <div class="w-full bg-gray-200 rounded-full h-2.5 mb-3">
      <div
        class="bg-blue-500 h-2.5 rounded-full transition-all duration-300"
        :style="{ width: `${progressPercent}%` }"
      />
    </div>

    <div class="flex items-center justify-between text-sm text-gray-500">
      <span class="truncate max-w-xs" :title="progress.currentFile">
        {{ fileName }}
      </span>
      <span>{{ progressPercent }}%</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useConversionStore, type ConversionProgress } from '@/stores/conversion'

const props = defineProps<{ progress: ConversionProgress }>()
const store = useConversionStore()

const progressPercent = computed(() =>
  props.progress.total > 0
    ? Math.round((props.progress.current / props.progress.total) * 100)
    : 0
)

const fileName = computed(() => {
  const parts = props.progress.currentFile.split(/[/\\]/)
  return parts[parts.length - 1] || props.progress.currentFile
})
</script>

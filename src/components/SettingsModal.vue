<template>
  <!-- Backdrop -->
  <div
    class="fixed inset-0 bg-black/40 z-40 flex items-center justify-center"
    @click.self="$emit('close')"
  >
    <div class="bg-white rounded-2xl shadow-xl w-full max-w-lg mx-4 p-6 z-50">
      <div class="flex items-center justify-between mb-5">
        <h2 class="text-lg font-semibold text-gray-800">设置</h2>
        <button
          class="text-gray-400 hover:text-gray-600 text-xl leading-none"
          @click="$emit('close')"
        >×</button>
      </div>

      <div class="space-y-5">
        <!-- Search depth -->
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">搜索深度（层数）</label>
          <input
            v-model.number="local.searchDepth"
            type="number"
            min="1"
            max="5"
            class="w-24 border border-gray-300 rounded-lg px-3 py-1.5 text-sm"
          />
          <p class="text-xs text-gray-400 mt-1">从学生文件夹向下搜索几层（默认 2）</p>
        </div>

        <!-- Concurrency -->
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">并发转换数</label>
          <input
            v-model.number="local.concurrency"
            type="number"
            min="1"
            max="8"
            class="w-24 border border-gray-300 rounded-lg px-3 py-1.5 text-sm"
          />
          <p class="text-xs text-gray-400 mt-1">1–8，根据机器配置自动推荐</p>
        </div>

        <!-- Output mode -->
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">输出位置</label>
          <div class="flex gap-4">
            <label class="flex items-center gap-2 text-sm">
              <input type="radio" v-model="local.outputConfig.mode" value="same" />
              原文件目录
            </label>
            <label class="flex items-center gap-2 text-sm">
              <input type="radio" v-model="local.outputConfig.mode" value="custom" />
              指定目录
            </label>
          </div>
          <div v-if="local.outputConfig.mode === 'custom'" class="mt-2 flex gap-2">
            <input
              v-model="local.outputConfig.customDir"
              type="text"
              placeholder="输出目录路径"
              class="flex-1 border border-gray-300 rounded-lg px-3 py-1.5 text-sm"
            />
            <button
              class="px-3 py-1.5 text-sm border rounded-lg hover:bg-gray-50"
              @click="selectOutputDir"
            >浏览</button>
          </div>
        </div>

        <!-- Existing PDF strategy -->
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">同名 PDF 已存在时</label>
          <select
            v-model="local.outputConfig.existingPdfStrategy"
            class="border border-gray-300 rounded-lg px-3 py-1.5 text-sm"
          >
            <option value="skip">跳过</option>
            <option value="overwrite">覆盖</option>
            <option value="rename">自动重命名</option>
          </select>
        </div>

        <!-- Include keywords -->
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">匹配关键词（逗号分隔）</label>
          <textarea
            v-model="includeKwStr"
            rows="2"
            class="w-full border border-gray-300 rounded-lg px-3 py-1.5 text-sm resize-none"
          />
        </div>

        <!-- Exclude keywords -->
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">排除关键词（逗号分隔）</label>
          <textarea
            v-model="excludeKwStr"
            rows="2"
            class="w-full border border-gray-300 rounded-lg px-3 py-1.5 text-sm resize-none"
          />
        </div>
      </div>

      <div class="flex justify-end gap-3 mt-6">
        <button
          class="px-4 py-2 text-sm border rounded-lg hover:bg-gray-50"
          @click="$emit('close')"
        >取消</button>
        <button
          class="px-5 py-2 text-sm bg-blue-500 text-white rounded-lg hover:bg-blue-600"
          @click="save"
        >保存</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { useConversionStore, type Settings } from '@/stores/conversion'

const emit = defineEmits<{ (e: 'close'): void }>()

const store = useConversionStore()

// Deep clone for local editing
const local = ref<Settings>(JSON.parse(JSON.stringify(store.settings)))

watch(
  () => store.settings,
  (s) => { local.value = JSON.parse(JSON.stringify(s)) },
  { deep: true }
)

const includeKwStr = computed({
  get: () => local.value.includeKeywords.join(', '),
  set: (v) => { local.value.includeKeywords = v.split(',').map((s) => s.trim()).filter(Boolean) },
})

const excludeKwStr = computed({
  get: () => local.value.excludeKeywords.join(', '),
  set: (v) => { local.value.excludeKeywords = v.split(',').map((s) => s.trim()).filter(Boolean) },
})

async function selectOutputDir() {
  const selected = await open({ directory: true, multiple: false })
  if (typeof selected === 'string') {
    local.value.outputConfig.customDir = selected
  }
}

function save() {
  store.settings = JSON.parse(JSON.stringify(local.value))
  emit('close')
}
</script>

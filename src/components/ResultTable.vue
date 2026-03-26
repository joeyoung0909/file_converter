<template>
  <div class="flex flex-col gap-3">
    <!-- Toolbar -->
    <div class="flex items-center justify-between">
      <div class="flex gap-2">
        <button
          class="px-3 py-1.5 text-sm border rounded-lg hover:bg-gray-50 transition-colors"
          @click="store.selectAll()"
        >全选</button>
        <button
          class="px-3 py-1.5 text-sm border rounded-lg hover:bg-gray-50 transition-colors"
          @click="store.deselectAll()"
        >全不选</button>
      </div>
      <span class="text-sm text-gray-500">
        共 {{ store.entries.length }} 个学生文件夹，已选 {{ store.selectedCount }} 个
      </span>
    </div>

    <!-- Table -->
    <div class="overflow-x-auto rounded-xl border border-gray-200 shadow-sm">
      <table class="w-full text-sm">
        <thead>
          <tr class="bg-gray-50 border-b border-gray-200 text-gray-600 text-left">
            <th class="w-10 px-3 py-3">
              <input type="checkbox" :checked="allChecked" @change="toggleAll" class="rounded" />
            </th>
            <th class="px-4 py-3 font-medium">学生文件夹</th>
            <th class="px-4 py-3 font-medium">识别到的论文文件</th>
            <th class="px-4 py-3 font-medium w-32">状态</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="(entry, idx) in store.entries"
            :key="entry.studentPath"
            class="border-b border-gray-100 last:border-0 hover:bg-gray-50/50"
            :class="!store.selections[idx]?.checked ? 'opacity-50' : ''"
          >
            <!-- Checkbox -->
            <td class="px-3 py-2">
              <input
                type="checkbox"
                v-model="store.selections[idx].checked"
                class="rounded"
              />
            </td>

            <!-- Student folder name -->
            <td class="px-4 py-2 font-medium text-gray-800">{{ entry.studentName }}</td>

            <!-- File selector -->
            <td class="px-4 py-2">
              <div v-if="entry.identifyResult.status === 'NotFound'">
                <span class="text-gray-400 italic text-xs">未找到</span>
              </div>
              <div v-else-if="entry.identifyResult.candidates.length === 1">
                <span class="text-gray-700">{{ entry.identifyResult.candidates[0].name }}</span>
                <span
                  v-if="entry.identifyResult.candidates[0].isLarge"
                  class="ml-2 text-xs text-amber-600 bg-amber-50 px-1.5 py-0.5 rounded"
                >
                  大文件 {{ formatSize(entry.identifyResult.candidates[0].size) }}
                </span>
              </div>
              <div v-else>
                <select
                  v-model="store.selections[idx].selectedFileIndex"
                  class="text-sm border border-gray-300 rounded px-2 py-1 bg-white w-full max-w-xs"
                >
                  <option
                    v-for="(file, fi) in entry.identifyResult.candidates"
                    :key="file.path"
                    :value="fi"
                  >
                    {{ file.name }}
                    {{ file.isLarge ? `（大文件 ${formatSize(file.size)}）` : '' }}
                  </option>
                </select>
              </div>
            </td>

            <!-- Status badge -->
            <td class="px-4 py-2">
              <StatusBadge :status="entry.identifyResult.status" />
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useConversionStore } from '@/stores/conversion'
import StatusBadge from './StatusBadge.vue'

const store = useConversionStore()

const allChecked = computed(() => store.selections.every((s) => s.checked))

function toggleAll() {
  if (allChecked.value) {
    store.deselectAll()
  } else {
    store.selectAll()
  }
}

function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024 / 1024).toFixed(1)}GB`
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)}MB`
  return `${(bytes / 1024).toFixed(0)}KB`
}
</script>

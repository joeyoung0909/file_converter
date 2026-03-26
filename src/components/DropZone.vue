<template>
  <div
    class="border-2 border-dashed rounded-xl p-10 text-center transition-colors cursor-pointer"
    :class="isDragOver
      ? 'border-blue-500 bg-blue-50'
      : 'border-gray-300 bg-white hover:border-blue-400 hover:bg-blue-50/40'"
    @dragover.prevent="isDragOver = true"
    @dragleave="isDragOver = false"
    @drop.prevent="onDrop"
    @click="openFolderDialog"
  >
    <div class="flex flex-col items-center gap-3">
      <svg class="w-14 h-14 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
          d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v9a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"/>
      </svg>
      <p class="text-lg font-medium text-gray-600">拖拽根文件夹到此处</p>
      <p class="text-sm text-gray-400">或</p>
      <button
        class="px-5 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 transition-colors font-medium"
        @click.stop="openFolderDialog"
      >
        选择文件夹
      </button>
      <p class="text-xs text-gray-400 mt-1">选择包含所有学生子文件夹的根目录</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'

const emit = defineEmits<{ (e: 'select', path: string): void }>()

const isDragOver = ref(false)

async function openFolderDialog() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: '选择包含学生子文件夹的根目录',
  })
  if (typeof selected === 'string') {
    emit('select', selected)
  }
}

function onDrop(event: DragEvent) {
  isDragOver.value = false
  const items = event.dataTransfer?.items
  if (!items) return
  for (const item of Array.from(items)) {
    if (item.kind === 'file') {
      const file = item.getAsFile()
      if (file) {
        // Tauri provides the path via webkitRelativePath or we use the file path directly
        const path = (file as any).path as string | undefined
        if (path) {
          emit('select', path)
          return
        }
      }
    }
  }
}
</script>

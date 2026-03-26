<template>
  <span
    class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium"
    :class="badgeClass"
  >
    {{ icon }} {{ label }}
  </span>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { IdentifyStatus } from '@/stores/conversion'

const props = defineProps<{ status: IdentifyStatus }>()

const config: Record<IdentifyStatus, { label: string; icon: string; cls: string }> = {
  Identified: { label: '已识别', icon: '✓', cls: 'bg-green-100 text-green-700' },
  MultipleCandidates: { label: '多候选', icon: '⚠', cls: 'bg-amber-100 text-amber-700' },
  NotFound: { label: '未找到', icon: '✗', cls: 'bg-red-100 text-red-600' },
  Unconfirmed: { label: '待确认', icon: '?', cls: 'bg-gray-100 text-gray-600' },
}

const icon = computed(() => config[props.status]?.icon ?? '')
const label = computed(() => config[props.status]?.label ?? props.status)
const badgeClass = computed(() => config[props.status]?.cls ?? '')
</script>

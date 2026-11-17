<template>
  <div class="footer">
    <span id="status" :class="{ err: store.isError }">{{ store.status || HINT }}</span>
    <span id="meta">{{ metaText }}</span>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useTileStore } from '../stores/tile';
import { unit } from '../composables/useGeometry';

const store = useTileStore();
const HINT = '左键拖动磁贴 · 右键拖动图片 · 方向键微调图片 · 滚轮缩放 · 中键删除磁贴';

const metaText = computed(() => {
  const g = unit(store.geom);
  const gt = Number.isInteger(g) ? String(g) : g.toFixed(1);
  return `基准 ${store.geom.medium} · 间隙 ${store.geom.gap} · 网格 ${gt} · 视口 ${Math.round(store.worldScale * 100)}% · 图片 ${Math.round(store.zoom * 100)}% · 磁贴 ${store.blocks.length}`;
});
</script>

<style scoped>
.footer {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 12px;
  color: var(--muted);
  font-size: 12px;
  min-height: 18px;
}
#status { flex: 0 1 auto; }
#status.err { color: var(--danger); }
#meta { flex: 0 0 auto; margin-left: auto; color: #767676; white-space: nowrap; }
</style>

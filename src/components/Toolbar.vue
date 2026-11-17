<template>
  <div class="toolbar">
    <button @click="onPick">选择图片</button>
    <span class="field">{{ store.image ? `${store.image.baseName}　${store.image.naturalW}×${store.image.naturalH}` : '未选择图片' }}</span>
    <span class="spacer" />
    <label class="field">基准 <input ref="mediumRef" type="number" min="40" max="400" step="1" :value="store.geom.medium" @change="onGeomChange"></label>
    <label class="field">间隙 <input ref="gapRef" type="number" min="0" max="40" step="1" :value="store.geom.gap" @change="onGeomChange"></label>
  </div>

  <div class="toolbar">
    <button v-for="kind in SLOT_ORDER" :key="kind" class="tile" @click="onAddTile(kind)">
      {{ KIND_TEXT[kind] }}磁贴<small>{{ sizeText(kind) }}</small>
    </button>
    <span class="spacer" />
    <button @click="store.clearAll(); store.setStatus('画布已清空')">清空画布</button>
    <button @click="onCleanup">清理开始菜单残留</button>
    <button class="primary" @click="emit('openPin')">添加到开始菜单</button>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { useTileStore } from '../stores/tile';
import { tileApi } from '../api';
import { SLOT_ORDER, KIND_TEXT, type TileSlot } from '../types';
import { sizeOf } from '../composables/useGeometry';

const store = useTileStore();
const emit = defineEmits<{ openPin: [] }>();
const mediumRef = ref<HTMLInputElement | null>(null);
const gapRef = ref<HTMLInputElement | null>(null);

function sizeText(kind: TileSlot): string {
  const { w, h } = sizeOf(kind, store.geom);
  return `${w}×${h}`;
}

async function onPick() {
  try {
    const picked = await tileApi.pickImage();
    if (!picked) return;
    // 只有图片真正加载成功（naturalW > 0）才认为"已载入"
    if (store.image && store.image.path === picked.path && store.image.naturalW > 0) {
      store.setStatus('这张图片已经载入');
      return;
    }
    store.setImage({
      path: picked.path, baseName: picked.baseName, url: picked.url,
      naturalW: 0, naturalH: 0,
    });
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    store.setStatus('选择图片失败：' + msg, true);
  }
}

function onAddTile(kind: TileSlot) {
  if (store.addTile(kind)) {
    const { w, h } = sizeOf(kind, store.geom);
    store.setStatus(`${KIND_TEXT[kind]}磁贴 ${w}×${h}`);
  }
}

async function onCleanup() {
  store.setStatus('正在清理开始菜单残留…');
  try {
    const result = await tileApi.cleanupTiles(null);
    const removed = result.removed;
    const failed = result.failed;
    if (removed.length === 0 && failed.length === 0) {
      store.setStatus('没有需要清理的残留磁贴');
    } else if (failed.length === 0) {
      store.setStatus(`已清理 ${removed.length} 个残留磁贴：${removed.join('、')}`);
    } else {
      store.setStatus(`已清理 ${removed.length} 个（${removed.join('、')}），${failed.length} 个失败（${failed.join('、')}）`, true);
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    store.setStatus('清理失败：' + msg, true);
  }
}

function onGeomChange() {
  const medium = Number(mediumRef.value?.value) || store.geom.medium;
  const gap = Number(gapRef.value?.value) || 0;
  store.setGeom(medium, gap);
}
</script>

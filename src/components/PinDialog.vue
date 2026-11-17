<template>
  <div id="pinDialog" v-show="!hidden" @mousedown.self="close">
    <div class="modal">
      <h3>添加到开始菜单</h3>
      <label class="row"><span>名称</span><input type="text" maxlength="40"
        v-model="name" placeholder="磁贴名称（磁贴上不显示）" @keydown.enter="confirm" @keydown.escape="close"></label>
      <label class="row check">
        <input type="checkbox" v-model="extraEnabled">
        <span>宽 / 大磁贴素材（原生菜单包身份 / Start11 用）</span>
      </label>
      <div class="pin-preview">
        <div class="shots">
          <div v-for="slot in visibleSlots" :key="slot" class="shot" :class="slot">
            <img :src="previews[slot]" alt="">
            <label>{{ PIN_SLOTS[slot].text }} {{ slotText(slot) }}</label>
          </div>
        </div>
        <div class="preview-text" v-html="previewText"></div>
      </div>
      <p class="note">
        图像铺满整块磁贴、磁贴上不显示名称。<b>小 / 中</b>原生开始菜单直接生效；
        <b>宽 / 大</b>通过注册「包身份」让原生开始菜单认这四档（需要 Windows 10 2004+ 和 Windows SDK 工具）。
      </p>
      <div class="modal-actions">
        <button @click="close">取消</button>
        <button class="primary" :disabled="busy" @click="confirm">{{ busy ? '处理中…' : '添加' }}</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue';
import { useTileStore } from '../stores/tile';
import { tileApi } from '../api';
import { SLOT_ORDER, PIN_SLOTS, PIN_ICON_SIZES, type TileSlot, type AddTilePayload } from '../types';
import { KIND_TEXT } from '../types';

const props = defineProps<{ canvasExpose: { renderTileImage: (w: number, h?: number) => string | null; renderTileCanvas: (w: number, h?: number) => HTMLCanvasElement | null; sampleBackground: () => string } }>();
const store = useTileStore();

const hidden = ref(true);
const name = ref('');
const extraEnabled = ref(true);
const busy = ref(false);
const previews = ref<Partial<Record<TileSlot, string>>>({});

const visibleSlots = computed(() =>
  extraEnabled.value ? SLOT_ORDER : SLOT_ORDER.filter((s) => s === 'small' || s === 'medium'),
);

function slotText(slot: TileSlot): string {
  const s = PIN_SLOTS[slot];
  return s.w === s.h ? `${s.w}` : `${s.w}×${s.h}`;
}

const previewText = computed(() => {
  const block = store.activeBlock;
  if (!block || !props.canvasExpose.renderTileCanvas(1, 1)) {
    return '<b>磁贴不在图片范围内</b><br>先把图片拖到磁贴下方';
  }
  return `来源 <b>${KIND_TEXT[block.kind]}磁贴 ${block.w}×${block.h}</b><br>宽 / 大通过包身份写入原生开始菜单（需 Windows 10 2004+ 和 SDK 工具）`;
});

function open() {
  if (!store.image) { store.setStatus('请先选择一张图片', true); return; }
  if (store.blocks.length === 0) { store.setStatus('请先添加一个磁贴', true); return; }
  name.value = store.image.baseName;
  hidden.value = false;
  nextTick(updatePreviews);
}

function close() { hidden.value = true; }

function updatePreviews() {
  const block = store.activeBlock;
  const next: Partial<Record<TileSlot, string>> = {};
  for (const slot of SLOT_ORDER) {
    const spec = PIN_SLOTS[slot];
    const url = block ? props.canvasExpose.renderTileImage(spec.w * 2, spec.h * 2) : null;
    if (url) next[slot] = url;
  }
  previews.value = next;
}

watch(extraEnabled, updatePreviews);

async function confirm() {
  const block = store.activeBlock;
  if (!block) return;
  busy.value = true;
  store.setStatus('正在写入开始菜单…');

  try {
    const tiles: AddTilePayload['tiles'] = {};
    for (const slot of SLOT_ORDER) {
      if (!extraEnabled.value && (slot === 'wide' || slot === 'large')) continue;
      const spec = PIN_SLOTS[slot];
      const v100 = props.canvasExpose.renderTileImage(spec.w, spec.h);
      const v200 = props.canvasExpose.renderTileImage(spec.w * 2, spec.h * 2);
      const variants = [
        v100 ? { scale: 100, dataUrl: v100 } : null,
        v200 ? { scale: 200, dataUrl: v200 } : null,
      ].filter(Boolean) as { scale: number; dataUrl: string }[];
      if (variants.length > 0) tiles[slot] = variants;
    }

    const icoImages = PIN_ICON_SIZES
      .map((size) => ({ size, dataUrl: props.canvasExpose.renderTileImage(size) }))
      .filter((item) => item.dataUrl) as { size: number; dataUrl: string }[];

    const pngDataUrl = props.canvasExpose.renderTileImage(256);
    if (!pngDataUrl || icoImages.length === 0 || !tiles.medium) {
      throw new Error('磁贴不在图片范围内，先把图片拖到磁贴下方');
    }

    const result = await tileApi.addStartTile({
      name: name.value || store.image!.baseName,
      pngDataUrl, icoImages, tiles,
      backgroundColor: props.canvasExpose.sampleBackground(),
      foregroundText: 'light',
    });

    if (!result?.ok) throw new Error(result?.error || '写入失败');

    close();
    if (result.pinnedViaPackage) {
      store.setStatus(`已添加「${result.name}」：四档尺寸原生可用。请在开始菜单 → 应用列表里找「${result.name}」→ 右键 → 固定到"开始"屏幕`);
    } else {
      const how = result.mode === 'fullbleed'
        ? (result.sparse?.ok ? '铺满磁贴、四档尺寸原生可用（包身份）' : `铺满磁贴、无名称（宽 / 大未注册${result.sparse?.error ? '：' + result.sparse.error : ''}）`)
        : '图标模式（启动器编译失败，已退回）';
      store.setStatus(`已添加「${result.name}」：${how}`);
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    store.setStatus('添加到开始菜单失败：' + msg, true);
  } finally {
    busy.value = false;
  }
}

defineExpose({ open, close });
</script>

<style scoped>
#pinDialog {
  position: fixed; inset: 0; z-index: 10;
  display: flex; align-items: center; justify-content: center;
  background: rgba(0, 0, 0, .55);
}
.modal {
  width: 440px; max-width: calc(100vw - 40px);
  padding: 16px 18px 14px;
  border: 1px solid var(--line); border-radius: 10px;
  background: var(--panel);
  box-shadow: 0 18px 48px rgba(0, 0, 0, .55);
  user-select: text;
}
.modal h3 { margin: 0 0 12px; font-size: 14px; font-weight: 500; }
.modal .row { display: flex; align-items: center; gap: 8px; margin-bottom: 10px; color: var(--muted); }
.modal .row > span { flex: 0 0 62px; }
.modal .row > input { flex: 1 1 auto; }
.modal .row.check { gap: 6px; margin-bottom: 12px; }
.modal .row.check > input { flex: 0 0 auto; width: 14px; height: 14px; accent-color: var(--accent); }
.modal .row.check > span { flex: 1 1 auto; font-size: 12px; color: var(--muted); }
.pin-preview {
  display: flex; flex-direction: column; gap: 10px;
  margin: 4px 0 10px; padding: 10px;
  border: 1px solid var(--line); border-radius: 8px; background: #161616;
}
.pin-preview .shots { display: flex; flex-wrap: wrap; align-items: flex-end; gap: 12px; }
.pin-preview .shot { display: flex; flex-direction: column; align-items: center; gap: 5px; }
.pin-preview .shot img { display: block; border-radius: 3px; background: #0d0d0d; }
.pin-preview .shot.small img { width: 40px; height: 40px; }
.pin-preview .shot.medium img { width: 80px; height: 80px; }
.pin-preview .shot.wide img { width: 120px; height: 58px; }
.pin-preview .shot.large img { width: 80px; height: 80px; }
.pin-preview .shot label { font-size: 11px; color: #8b8b8b; }
.preview-text { min-width: 0; font-size: 12px; color: var(--muted); line-height: 1.7; }
.preview-text :deep(b) { color: var(--text); font-weight: 500; }
.note { margin: 0 0 12px; font-size: 12px; line-height: 1.6; color: #8b8b8b; }
.note :deep(b) { color: var(--text); }
.modal-actions { display: flex; justify-content: flex-end; gap: 8px; }
</style>

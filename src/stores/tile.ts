/**
 * Pinia store —— 集中管理画布状态，替代 renderer.js 的全局变量
 * 状态：blocks / selected / pan / zoom / worldScale / image / geom
 */
import { defineStore } from 'pinia';
import { ref, computed, shallowRef } from 'vue';
import type { Block, Geometry, LoadedImage, TileSlot } from '../types';
import {
  DEFAULT_GEOM, clampGeom, fitScale, findSpot, sizeOf, snapBlock, unit,
  MIN_WORLD_SCALE, clamp, MIN_ZOOM,
} from '../composables/useGeometry';

const STORAGE_KEY = 'tilecropper.settings.v4';

export const useTileStore = defineStore('tile', () => {
  // ---------- 状态 ----------
  const blocks = ref<Block[]>([]);
  const selectedId = ref<number | null>(null);
  const pan = ref({ x: 0, y: 0 });
  const zoom = ref(1);
  const worldScale = ref(1);
  const image = shallowRef<LoadedImage | null>(null);
  const geom = ref<Geometry>({ ...DEFAULT_GEOM });
  const view = ref({ cssW: 0, cssH: 0, dpr: 1 });
  const status = ref('');
  const isError = ref(false);
  const nextId = ref(1);

  // ---------- 计算 ----------
  const selected = computed(() =>
    blocks.value.find((b) => b.id === selectedId.value) ?? null,
  );

  const activeBlock = computed(() =>
    selected.value ?? blocks.value[blocks.value.length - 1] ?? null,
  );

  const worldW = computed(() => view.value.cssW / worldScale.value);
  const worldH = computed(() => view.value.cssH / worldScale.value);

  const gridUnit = computed(() => unit(geom.value));

  // ---------- 动作 ----------
  function setView(cssW: number, cssH: number, dpr: number) {
    view.value = { cssW, cssH, dpr };
  }

  function setStatus(text: string, error = false) {
    status.value = text;
    isError.value = error;
  }

  function addTile(kind: TileSlot): boolean {
    const { w, h } = sizeOf(kind, geom.value);

    // 默认位置：磁贴中心对齐图片显示区域中心，便于取景
    let preferX: number;
    let preferY: number;
    if (image.value && image.value.naturalW > 0 && image.value.naturalH > 0) {
      const imgCx = pan.value.x + (image.value.naturalW * zoom.value) / 2;
      const imgCy = pan.value.y + (image.value.naturalH * zoom.value) / 2;
      preferX = imgCx - w / 2;
      preferY = imgCy - h / 2;
    } else {
      const ref0 = selected.value ?? blocks.value[blocks.value.length - 1];
      preferX = ref0 ? ref0.x + ref0.w + geom.value.gap : geom.value.gap;
      preferY = ref0 ? ref0.y : geom.value.gap;
    }

    let spot = findSpot(w, h, preferX, preferY, blocks.value, worldW.value, worldH.value, geom.value);
    while (!spot && worldScale.value > MIN_WORLD_SCALE) {
      worldScale.value = Math.max(MIN_WORLD_SCALE, worldScale.value * 0.9);
      spot = findSpot(w, h, preferX, preferY, blocks.value, worldW.value, worldH.value, geom.value);
    }

    if (!spot) {
      setStatus('画布放不下了：先删掉一些磁贴，或者把窗口拉大', true);
      return false;
    }

    const block: Block = { id: nextId.value++, kind, w, h, x: spot.x, y: spot.y };
    blocks.value.push(block);
    selectedId.value = block.id;
    worldScale.value = Math.min(worldScale.value, fitScale(blocks.value, view.value.cssW, view.value.cssH, geom.value));
    return true;
  }

  function removeBlock(id: number) {
    blocks.value = blocks.value.filter((b) => b.id !== id);
    if (selectedId.value === id) selectedId.value = null;
    refreshScale();
  }

  function selectBlock(id: number | null) {
    selectedId.value = id;
    if (id !== null) {
      // 选中的块移到数组末尾（渲染在最上层）
      const idx = blocks.value.findIndex((b) => b.id === id);
      if (idx >= 0 && idx < blocks.value.length - 1) {
        const [b] = blocks.value.splice(idx, 1);
        blocks.value.push(b);
      }
    }
  }

  function clearAll() {
    blocks.value = [];
    selectedId.value = null;
    refreshScale();
  }

  function refreshScale() {
    worldScale.value = fitScale(blocks.value, view.value.cssW, view.value.cssH, geom.value);
  }

  /** 拖拽中更新位置（不吸附） */
  function dragBlock(id: number, x: number, y: number) {
    const b = blocks.value.find((b) => b.id === id);
    if (b) { b.x = x; b.y = y; }
  }

  /** 拖拽结束，吸附网格 */
  function dropBlock(id: number) {
    const b = blocks.value.find((b) => b.id === id);
    if (!b) return;
    snapBlock(b, blocks.value, worldW.value, worldH.value, geom.value);
  }

  function setImage(img: LoadedImage | null) {
    image.value = img;
  }

  function fitImageToView() {
    if (!image.value || !image.value.naturalW || !image.value.naturalH) return;
    const spanW = Math.max(1, worldW.value - geom.value.gap);
    const spanH = Math.max(1, worldH.value - geom.value.gap);
    zoom.value = clamp(
      Math.min(1, spanW / image.value.naturalW, spanH / image.value.naturalH),
      MIN_ZOOM, 1,
    );
    pan.value.x = geom.value.gap;
    pan.value.y = geom.value.gap;
  }

  function setGeom(medium: number, gap: number) {
    geom.value = clampGeom({ medium, gap });
    blocks.value = [];
    selectedId.value = null;
    refreshScale();
    if (image.value) fitImageToView();
    persist();
  }

  function persist() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(geom.value));
    } catch { /* 忽略 */ }
  }

  function restoreSettings() {
    try {
      const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) || 'null');
      if (saved) {
        geom.value = clampGeom(saved);
      }
    } catch { /* 用默认值 */ }
  }

  return {
    blocks, selectedId, selected, activeBlock,
    pan, zoom, worldScale, worldW, worldH, gridUnit,
    image, geom, view, status, isError,
    setView, setStatus, addTile, removeBlock, selectBlock,
    clearAll, refreshScale, dragBlock, dropBlock,
    setImage, fitImageToView, setGeom, persist, restoreSettings,
  };
});

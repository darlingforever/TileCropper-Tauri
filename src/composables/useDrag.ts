/**
 * 拖拽交互 —— 移植自 renderer.js 的 mousedown/mousemove/mouseup/wheel/keydown
 * 封装为可复用的 composable
 */
import { type Ref, onMounted, onUnmounted } from 'vue';
import type { useTileStore } from '../stores/tile';
import { clamp, MIN_ZOOM, MAX_ZOOM } from './useGeometry';

type Store = ReturnType<typeof useTileStore>;

export function useDrag(
  stageRef: Ref<HTMLElement | null>,
  store: Store,
  callbacks: {
    onDraw: () => void;
    onUpdateImage: () => void;
    onSnap: () => void;
  },
) {
  let dragMode: 'tile' | 'pan' | null = null;
  let dragOffset = { x: 0, y: 0 };
  let draggingId: number | null = null;

  function toWorld(clientX: number, clientY: number) {
    const el = stageRef.value!;
    const rect = el.getBoundingClientRect();
    return {
      x: (clientX - rect.left) / store.worldScale,
      y: (clientY - rect.top) / store.worldScale,
    };
  }

  function blockAt(x: number, y: number) {
    const blocks = store.blocks;
    for (let i = blocks.length - 1; i >= 0; i--) {
      const b = blocks[i];
      if (x >= b.x && x <= b.x + b.w && y >= b.y && y <= b.y + b.h) return b;
    }
    return null;
  }

  function onMouseDown(e: MouseEvent) {
    const el = stageRef.value;
    if (!el) return;
    const point = toWorld(e.clientX, e.clientY);

    if (e.button === 2) {
      dragMode = 'pan';
      dragOffset = { x: point.x - store.pan.x, y: point.y - store.pan.y };
      e.preventDefault();
      return;
    }

    if (e.button === 1) {
      e.preventDefault();
      const target = blockAt(point.x, point.y);
      if (target) store.removeBlock(target.id);
      callbacks.onDraw();
      return;
    }

    if (e.button === 0) {
      const target = blockAt(point.x, point.y);
      if (!target) {
        if (store.selectedId !== null) {
          store.selectBlock(null);
          callbacks.onDraw();
        }
        return;
      }
      store.selectBlock(target.id);
      draggingId = target.id;
      dragMode = 'tile';
      dragOffset = { x: point.x - target.x, y: point.y - target.y };
      document.body.classList.add('dragging');
      callbacks.onDraw();
      e.preventDefault();
    }
  }

  function onMouseMove(e: MouseEvent) {
    if (!dragMode) return;
    const point = toWorld(e.clientX, e.clientY);

    if (dragMode === 'tile' && draggingId !== null) {
      store.dragBlock(draggingId, point.x - dragOffset.x, point.y - dragOffset.y);
      callbacks.onDraw();
    } else if (dragMode === 'pan') {
      store.pan.x = point.x - dragOffset.x;
      store.pan.y = point.y - dragOffset.y;
      callbacks.onUpdateImage();
    }
  }

  function onMouseUp() {
    if (dragMode === 'tile' && draggingId !== null) {
      store.dropBlock(draggingId);
      callbacks.onDraw();
    }
    dragMode = null;
    draggingId = null;
    document.body.classList.remove('dragging');
  }

  function onWheel(e: WheelEvent) {
    if (!store.image) return;
    e.preventDefault();
    const point = toWorld(e.clientX, e.clientY);
    const imgX = (point.x - store.pan.x) / store.zoom;
    const imgY = (point.y - store.pan.y) / store.zoom;
    const next = clamp(store.zoom * (e.deltaY > 0 ? 0.9 : 1.1), MIN_ZOOM, MAX_ZOOM);
    if (next === store.zoom) return;
    store.zoom = next;
    store.pan.x = point.x - imgX * store.zoom;
    store.pan.y = point.y - imgY * store.zoom;
    callbacks.onUpdateImage();
  }

  function onKeydown(e: KeyboardEvent) {
    const tag = (e.target as HTMLElement)?.tagName;
    if (tag === 'INPUT' || tag === 'SELECT') return;

    if (e.key === 'Escape' && store.selectedId !== null) {
      store.selectBlock(null);
      callbacks.onDraw();
    }
    if ((e.key === 'Delete' || e.key === 'Backspace') && store.selectedId !== null) {
      e.preventDefault();
      store.removeBlock(store.selectedId);
      callbacks.onDraw();
    }

    const step = e.shiftKey ? 10 : 1;
    const delta: Record<string, [number, number]> = {
      ArrowLeft: [-step, 0], ArrowRight: [step, 0],
      ArrowUp: [0, -step], ArrowDown: [0, step],
    };
    const d = delta[e.key];
    if (d && store.image) {
      e.preventDefault();
      store.pan.x += d[0];
      store.pan.y += d[1];
      callbacks.onUpdateImage();
    }
  }

  onMounted(() => {
    const el = stageRef.value;
    if (!el) return;
    el.addEventListener('mousedown', onMouseDown);
    el.addEventListener('wheel', onWheel, { passive: false });
    el.addEventListener('contextmenu', (e) => e.preventDefault());
    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
    window.addEventListener('keydown', onKeydown);
  });

  onUnmounted(() => {
    const el = stageRef.value;
    el?.removeEventListener('mousedown', onMouseDown);
    el?.removeEventListener('wheel', onWheel);
    window.removeEventListener('mousemove', onMouseMove);
    window.removeEventListener('mouseup', onMouseUp);
    window.removeEventListener('keydown', onKeydown);
  });
}

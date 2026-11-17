/**
 * Canvas 绘制 + 磁贴裁切 —— 移植自 renderer.js 的 draw/renderTileCanvas
 * 职责：画遮罩、画选中框、从图片裁出磁贴区域
 */
import { type Ref } from 'vue';
import type { Block, LoadedImage } from '../types';
import type { useTileStore } from '../stores/tile';

type Store = ReturnType<typeof useTileStore>;

export function useCanvas(
  canvasRef: Ref<HTMLCanvasElement | null>,
  imgRef: Ref<HTMLImageElement | null>,
  store: Store,
) {
  /** 更新底层 <img> 的 transform（平移 + 缩放） */
  function updateImage() {
    const img = imgRef.value;
    if (!img || !img.src) return;
    const tx = store.pan.x * store.worldScale;
    const ty = store.pan.y * store.worldScale;
    img.style.transform = `translate(${tx}px, ${ty}px) scale(${store.zoom * store.worldScale})`;
  }

  /** 重绘遮罩层 */
  function draw() {
    const canvas = canvasRef.value;
    if (!canvas) return;
    const ctx = canvas.getContext('2d')!;
    const { dpr } = store.view;
    const ws = store.worldScale;
    const hasImg = Boolean(store.image);

    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.fillStyle = hasImg ? 'rgba(16, 16, 16, 0.55)' : '#2c2c2c';
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    ctx.save();
    ctx.setTransform(dpr * ws, 0, 0, dpr * ws, 0, 0);

    // 挖空磁贴区域
    if (hasImg) {
      ctx.globalCompositeOperation = 'destination-out';
      for (const b of store.blocks) ctx.fillRect(b.x, b.y, b.w, b.h);
      ctx.globalCompositeOperation = 'source-over';
    } else {
      ctx.fillStyle = '#383838';
      for (const b of store.blocks) ctx.fillRect(b.x, b.y, b.w, b.h);
    }

    // 框线
    ctx.lineWidth = 1 / ws;
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.32)';
    for (const b of store.blocks) {
      if (b.id === store.selectedId) continue;
      ctx.strokeRect(b.x, b.y, b.w, b.h);
    }

    const sel = store.selected;
    if (sel) {
      ctx.lineWidth = 2 / ws;
      ctx.strokeStyle = '#4caf50';
      ctx.strokeRect(sel.x, sel.y, sel.w, sel.h);
    }
    ctx.restore();
  }

  /** 画布尺寸适配 DPR */
  function resizeCanvas(cssW: number, cssH: number) {
    const canvas = canvasRef.value;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    store.setView(Math.max(1, cssW), Math.max(1, cssH), dpr);
    canvas.style.width = cssW + 'px';
    canvas.style.height = cssH + 'px';
    canvas.width = Math.round(cssW * dpr);
    canvas.height = Math.round(cssH * dpr);
    store.refreshScale();
    updateImage();
    draw();
  }

  /** 磁贴区域 -> 原图像素坐标 */
  function blockSourceRect(block: Block, img: HTMLImageElement, image: LoadedImage) {
    const layoutW = img.offsetWidth || image.naturalW;
    const layoutH = img.offsetHeight || image.naturalH;
    const kx = layoutW > 0 ? image.naturalW / layoutW : 1;
    const ky = layoutH > 0 ? image.naturalH / layoutH : 1;
    return {
      x: ((block.x - store.pan.x) / store.zoom) * kx,
      y: ((block.y - store.pan.y) / store.zoom) * ky,
      w: (block.w / store.zoom) * kx,
      h: (block.h / store.zoom) * ky,
    };
  }

  /** 把磁贴区域按目标宽高比居中裁切并缩放，返回 canvas */
  function renderTileCanvas(w: number, h = w): HTMLCanvasElement | null {
    const img = imgRef.value;
    if (!img || !img.naturalWidth || !store.image) return null;
    const block = store.activeBlock;
    if (!block) return null;

    const rect = blockSourceRect(block, img, store.image);
    if (!(rect.w > 0) || !(rect.h > 0)) return null;

    const aspect = w / h;
    let cw = rect.h * aspect;
    let ch = rect.h;
    if (cw > rect.w) { cw = rect.w; ch = rect.w / aspect; }
    const sx = rect.x + (rect.w - cw) / 2;
    const sy = rect.y + (rect.h - ch) / 2;

    // 裁剪到图片边界
    const ix0 = Math.max(0, sx);
    const iy0 = Math.max(0, sy);
    const ix1 = Math.min(store.image.naturalW, sx + cw);
    const iy1 = Math.min(store.image.naturalH, sy + ch);
    if (ix1 <= ix0 || iy1 <= iy0) return null;

    const tc = document.createElement('canvas');
    tc.width = Math.max(1, Math.round(w));
    tc.height = Math.max(1, Math.round(h));
    const tctx = tc.getContext('2d')!;
    tctx.imageSmoothingEnabled = true;
    tctx.imageSmoothingQuality = 'high';
    tctx.drawImage(
      img,
      ix0, iy0, ix1 - ix0, iy1 - iy0,
      ((ix0 - sx) / cw) * w, ((iy0 - sy) / ch) * h,
      ((ix1 - ix0) / cw) * w, ((iy1 - iy0) / ch) * h,
    );
    return tc;
  }

  function renderTileImage(w: number, h = w): string | null {
    const c = renderTileCanvas(w, h);
    return c ? c.toDataURL('image/png') : null;
  }

  /** 取四角平均色作为磁贴底色 */
  function sampleBackground(): string {
    try {
      const probe = renderTileCanvas(8);
      if (!probe) return '#1f1f1f';
      const data = probe.getContext('2d')!.getImageData(0, 0, 8, 8).data;
      let r = 0, g = 0, b = 0, n = 0;
      for (const [x, y] of [[0, 0], [7, 0], [0, 7], [7, 7]]) {
        const i = (y * 8 + x) * 4;
        if (data[i + 3] < 8) continue;
        r += data[i]; g += data[i + 1]; b += data[i + 2]; n++;
      }
      if (n === 0) return '#1f1f1f';
      const hex = (v: number) => Math.round(v / n).toString(16).padStart(2, '0');
      return `#${hex(r)}${hex(g)}${hex(b)}`;
    } catch {
      return '#1f1f1f';
    }
  }

  return {
    updateImage, draw, resizeCanvas,
    renderTileCanvas, renderTileImage, sampleBackground,
  };
}

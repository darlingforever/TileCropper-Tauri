/**
 * 磁贴网格几何 —— 纯函数，无副作用，方便单测
 * 移植自 renderer.js 的 unit/edge/sizeOf/collides/findSpot/snapBlock
 */
import { type Block, type Geometry, type TileSlot, KIND_CELLS } from '../types';

export const MIN_WORLD_SCALE = 0.2;
export const MIN_ZOOM = 0.05;
export const MAX_ZOOM = 8;

export const DEFAULT_GEOM: Geometry = { medium: 150, gap: 0 };

/** 网格单位 u = (基准 + 间隙) / 2 */
export function unit(geom: Geometry): number {
  return (geom.medium + geom.gap) / 2;
}

/** n 个网格单位的实际边长（减去尾部间隙） */
export function edge(cells: number, geom: Geometry): number {
  return Math.max(1, Math.round(cells * unit(geom) - geom.gap));
}

export function sizeOf(kind: TileSlot, geom: Geometry): { w: number; h: number } {
  const [cols, rows] = KIND_CELLS[kind];
  return { w: edge(cols, geom), h: edge(rows, geom) };
}

export function clamp(v: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, v));
}

/** 碰撞检测：判断 x,y,w,h 是否与已有 block 重叠（带间隙膨胀） */
export function collides(
  x: number, y: number, w: number, h: number,
  blocks: Block[], ignore?: Block, geom = DEFAULT_GEOM,
): boolean {
  const gap = geom.gap;
  for (const b of blocks) {
    if (b === ignore) continue;
    if (x < b.x + b.w + gap && b.x < x + w + gap &&
        y < b.y + b.h + gap && b.y < y + h + gap) {
      return true;
    }
  }
  return false;
}

/** 在网格上找最近的可用位置 */
export function findSpot(
  w: number, h: number, preferX: number, preferY: number,
  blocks: Block[], worldW: number, worldH: number, geom = DEFAULT_GEOM,
): { x: number; y: number } | null {
  const gap = geom.gap;
  const step = unit(geom);
  const maxX = worldW - gap - w;
  const maxY = worldH - gap - h;
  if (maxX < gap || maxY < gap) return null;

  let best: { x: number; y: number } | null = null;
  let bestDist = Infinity;

  for (let y = gap; y <= maxY + 0.5; y += step) {
    for (let x = gap; x <= maxX + 0.5; x += step) {
      if (collides(x, y, w, h, blocks, undefined, geom)) continue;
      const dist = Math.abs(x - preferX) + Math.abs(y - preferY);
      if (dist < bestDist) {
        bestDist = dist;
        best = { x, y };
      }
    }
  }
  return best;
}

/** 计算自适应缩放，让所有磁贴适配视口 */
export function fitScale(
  blocks: Block[], cssW: number, cssH: number, geom = DEFAULT_GEOM,
): number {
  const gap = geom.gap;
  let maxX = gap;
  let maxY = gap;
  for (const b of blocks) {
    maxX = Math.max(maxX, b.x + b.w);
    maxY = Math.max(maxY, b.y + b.h);
  }
  const needW = maxX + gap;
  const needH = maxY + gap;
  const scale = Math.min(1, cssW / needW, cssH / needH);
  return clamp(scale, MIN_WORLD_SCALE, 1);
}

/** 松手后吸附到最近的可用网格位置（环形搜索） */
export function snapBlock(
  block: Block, blocks: Block[], worldW: number, worldH: number, geom = DEFAULT_GEOM,
): void {
  const gap = geom.gap;
  const step = unit(geom);
  const baseI = Math.round((block.x - gap) / step);
  const baseJ = Math.round((block.y - gap) / step);
  const maxX = worldW - gap - block.w;
  const maxY = worldH - gap - block.h;

  for (let radius = 0; radius <= 40; radius++) {
    const candidates: { i: number; j: number; d: number }[] = [];
    for (let dj = -radius; dj <= radius; dj++) {
      for (let di = -radius; di <= radius; di++) {
        if (radius > 0 && Math.abs(di) !== radius && Math.abs(dj) !== radius) continue;
        candidates.push({ i: baseI + di, j: baseJ + dj, d: Math.abs(di) + Math.abs(dj) });
      }
    }
    candidates.sort((a, b) => a.d - b.d);

    for (const c of candidates) {
      const x = gap + c.i * step;
      const y = gap + c.j * step;
      if (x < gap - 0.01 || y < gap - 0.01) continue;
      if (x > maxX + 0.5 || y > maxY + 0.5) continue;
      if (collides(x, y, block.w, block.h, blocks, block, geom)) continue;
      block.x = x;
      block.y = y;
      return;
    }
  }
}

/** 限制几何参数到合法范围 */
export function clampGeom(geom: Geometry): Geometry {
  return {
    medium: clamp(Math.round(geom.medium) || DEFAULT_GEOM.medium, 40, 400),
    gap: clamp(Math.round(geom.gap) || 0, 0, 40),
  };
}

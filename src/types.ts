/**
 * 前后端共享类型定义 —— IPC 契约
 * Rust 端 (src-tauri/src/types.rs) 保持镜像结构
 */

/** 磁贴档位 */
export type TileSlot = 'small' | 'medium' | 'wide' | 'large';

/** 磁贴占几格（列, 行） */
export const KIND_CELLS: Record<TileSlot, readonly [number, number]> = {
  small: [1, 1],
  medium: [2, 2],
  wide: [4, 2],
  large: [4, 4],
};

export const KIND_TEXT: Record<TileSlot, string> = {
  small: '小',
  medium: '中',
  wide: '宽',
  large: '大',
};

export const SLOT_ORDER: TileSlot[] = ['small', 'medium', 'wide', 'large'];

/** 画布上的一个磁贴 */
export interface Block {
  id: number;
  kind: TileSlot;
  x: number;
  y: number;
  w: number;
  h: number;
}

/** 载入的图片状态 */
export interface LoadedImage {
  path: string;
  baseName: string;
  url: string;
  naturalW: number;
  naturalH: number;
}

/** 几何参数 */
export interface Geometry {
  medium: number;
  gap: number;
}

/** Pin 素材档位规格 */
export interface PinSlotSpec {
  w: number;
  h: number;
  text: string;
}

export const PIN_SLOTS: Record<TileSlot, PinSlotSpec> = {
  small: { w: 70, h: 70, text: '小' },
  medium: { w: 150, h: 150, text: '中' },
  wide: { w: 310, h: 150, text: '宽' },
  large: { w: 310, h: 310, text: '大' },
};

export const PIN_ICON_SIZES = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256];

/** 单档磁贴的缩放变体 */
export interface TileVariant {
  scale: number;
  dataUrl: string;
}

/** 图标条目 */
export interface IcoImage {
  size: number;
  dataUrl: string;
}

/** 渲染端发给 Rust 的 payload */
export interface AddTilePayload {
  name: string;
  pngDataUrl: string;
  icoImages: IcoImage[];
  tiles: Partial<Record<TileSlot, TileVariant[]>>;
  backgroundColor: string;
  foregroundText: 'light' | 'dark';
}

/** Rust 返回的结果 */
export interface AddTileResult {
  ok: boolean;
  error?: string;
  name?: string;
  mode?: 'fullbleed' | 'icon';
  pin?: 'ok' | 'noverb' | 'error';
  pinnedViaPackage?: boolean;
  sparse?: { ok: boolean; error?: string; reason?: string };
}

export interface CleanupResult {
  removed: string[];
  failed: string[];
  scanned: number;
}

/** IPC 调用封装 */
export interface TileApi {
  pickImage(): Promise<{ path: string; url: string; baseName: string } | null>;
  addStartTile(payload: AddTilePayload): Promise<AddTileResult>;
  cleanupTiles(name: string | null): Promise<CleanupResult>;
}

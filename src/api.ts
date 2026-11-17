/**
 * Tauri IPC 封装 —— 替代 Electron 的 window.tileAPI
 * 通过 Tauri invoke 调用 Rust 命令
 */
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { AddTilePayload, AddTileResult, CleanupResult, TileApi } from './types';

export const tileApi: TileApi = {
  async pickImage() {
    const selected = await open({
      title: '选择图片',
      multiple: false,
      filters: [{ name: '图片', extensions: ['png', 'jpg', 'jpeg', 'gif', 'bmp', 'webp'] }],
    });
    if (!selected || typeof selected !== 'string') return null;

    const sep = /[\\/]/;
    const parts = selected.split(sep);
    const fileName = parts[parts.length - 1];
    const baseName = fileName.replace(/\.[^.]+$/, '');

    // 用 data URL 而非 convertFileSrc —— asset 协议跨域会污染 Canvas，
    // 导致 toDataURL 抛 SecurityError；data URL 与页面同源，不污染。
    const url = await invoke<string>('read_image_as_data_url', { path: selected });

    return {
      path: selected,
      url,
      baseName,
    };
  },

  addStartTile(payload: AddTilePayload) {
    return invoke<AddTileResult>('add_start_tile', { payload });
  },

  cleanupTiles(name: string | null) {
    return invoke<CleanupResult>('cleanup_tiles', { name });
  },
};

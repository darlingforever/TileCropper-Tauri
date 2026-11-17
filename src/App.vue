<template>
  <Toolbar @open-pin="pinDialog?.open()" />
  <TileCanvas ref="tileCanvas" />
  <StatusBar />
  <PinDialog ref="pinDialog" :canvas-expose="canvasExpose" />
</template>

<script setup lang="ts">
import { ref, shallowRef, onMounted } from 'vue';
import Toolbar from './components/Toolbar.vue';
import TileCanvas from './components/TileCanvas.vue';
import StatusBar from './components/StatusBar.vue';
import PinDialog from './components/PinDialog.vue';

const tileCanvas = ref<InstanceType<typeof TileCanvas> | null>(null);
const pinDialog = ref<InstanceType<typeof PinDialog> | null>(null);

// PinDialog 需要从 TileCanvas 拿裁图能力，这里桥接
const canvasExpose = shallowRef({
  renderTileImage: (_w: number, _h?: number): string | null => null,
  renderTileCanvas: (_w: number, _h?: number): HTMLCanvasElement | null => null,
  sampleBackground: (): string => '#1f1f1f',
});

onMounted(() => {
  if (tileCanvas.value) {
    canvasExpose.value = {
      renderTileImage: tileCanvas.value.renderTileImage,
      renderTileCanvas: tileCanvas.value.renderTileCanvas,
      sampleBackground: tileCanvas.value.sampleBackground,
    };
  }
});
</script>

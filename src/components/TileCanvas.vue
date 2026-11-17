<template>
  <div ref="stageRef" id="stage">
    <img ref="imgRef" id="imageView" alt="" draggable="false"
      :src="store.image?.url" :class="{ loaded: imgLoaded }"
      @load="onImgLoad" @error="onImgError" />
    <canvas ref="canvasRef" id="tiles"></canvas>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue';
import { useTileStore } from '../stores/tile';
import { useCanvas } from '../composables/useCanvas';
import { useDrag } from '../composables/useDrag';

const store = useTileStore();
const stageRef = ref<HTMLElement | null>(null);
const canvasRef = ref<HTMLCanvasElement | null>(null);
const imgRef = ref<HTMLImageElement | null>(null);
const imgLoaded = ref(false);

const { updateImage, draw, resizeCanvas, renderTileImage, renderTileCanvas, sampleBackground } =
  useCanvas(canvasRef, imgRef, store);

useDrag(stageRef, store, {
  onDraw: draw,
  onUpdateImage: updateImage,
  onSnap: draw,
});

defineExpose({ renderTileImage, renderTileCanvas, sampleBackground });

// 图片加载成功
function onImgLoad() {
  const img = imgRef.value;
  if (!img || !store.image) return;
  store.setImage({
    ...store.image,
    naturalW: img.naturalWidth,
    naturalH: img.naturalHeight,
  });
  imgLoaded.value = true;
  store.fitImageToView();
  updateImage();
  draw();
  store.setStatus('图片已载入：右键拖动或方向键微调位置，滚轮缩放');
}

// 图片加载失败：清掉状态，避免第二次选同一张时误判"已载入"
function onImgError() {
  imgLoaded.value = false;
  store.setImage(null);
  store.setStatus('图片加载失败：换一张图片，或确认文件没被移动', true);
}

// 选了新图片时先隐藏旧图，等 load 事件再显示
watch(() => store.image?.path, (newPath, oldPath) => {
  if (newPath !== oldPath) {
    imgLoaded.value = false;
  }
});

watch(() => [store.blocks, store.selectedId, store.worldScale], draw, { deep: true });

let ro: ResizeObserver | null = null;
onMounted(() => {
  store.restoreSettings();
  const el = stageRef.value!;
  if (typeof ResizeObserver === 'function') {
    ro = new ResizeObserver(() => {
      resizeCanvas(el.clientWidth, el.clientHeight);
    });
    ro.observe(el);
  }
  window.addEventListener('resize', onWinResize);
  nextTick(() => resizeCanvas(el.clientWidth, el.clientHeight));
});

onUnmounted(() => {
  ro?.disconnect();
  window.removeEventListener('resize', onWinResize);
});

function onWinResize() {
  const el = stageRef.value;
  if (el) resizeCanvas(el.clientWidth, el.clientHeight);
}
</script>

<style scoped>
#stage {
  position: relative;
  flex: 1 1 auto;
  min-height: 0;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #161616;
}
#imageView {
  position: absolute; top: 0; left: 0;
  transform-origin: 0 0;
  max-width: none; pointer-events: none;
  display: none; -webkit-user-drag: none;
}
#imageView.loaded { display: block; }
#tiles { position: absolute; top: 0; left: 0; pointer-events: none; }
</style>

<script setup lang="ts">
defineProps<{
  url: string;
  revision: number;
}>();
</script>

<template>
  <iframe
    :key="revision"
    class="dsh-frame"
    :src="url"
    title="DeepSeek Harness"
    allow="clipboard-read; clipboard-write"
    sandbox="allow-downloads allow-forms allow-modals allow-popups allow-popups-to-escape-sandbox allow-same-origin allow-scripts"
    referrerpolicy="no-referrer"
  />
</template>

<style scoped>
.dsh-frame {
  width: 100%;
  height: 100%;
  border: 0;
  background: var(--color-background);
}
</style>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
const { t } = useI18n()
</script>

<template>
  <div class="tool-placeholder">
    <svg class="ph-icon" width="88" height="88" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
      <path d="M4 8V5a1 1 0 0 1 1-1h3" />
      <path d="M16 4h3a1 1 0 0 1 1 1v3" />
      <path d="M20 16v3a1 1 0 0 1-1 1h-3" />
      <path d="M8 20H5a1 1 0 0 1-1-1v-3" />
      <path d="M8 10h8M8 14h5" />
    </svg>
    <h1>{{ t('placeholder.ocrTitle') }}</h1>
    <p>{{ t('placeholder.ocrDesc') }}</p>
    <span class="badge">{{ t('placeholder.comingSoon') }}</span>
  </div>
</template>

<style scoped>
.tool-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 14px;
  color: var(--text-muted);
  text-align: center;
  padding: 40px;
}

.ph-icon {
  color: var(--accent);
  opacity: 0.55;
}

.tool-placeholder h1 {
  font-size: 26px;
  font-weight: 700;
  margin: 0;
  color: var(--text);
  letter-spacing: -0.5px;
}

.tool-placeholder p {
  font-size: 14px;
  max-width: 460px;
  line-height: 1.6;
  margin: 0;
}

.badge {
  margin-top: 6px;
  padding: 4px 14px;
  border-radius: 999px;
  font-size: 12px;
  color: var(--accent);
  background: var(--surface);
  border: 1px solid var(--border);
  letter-spacing: 0.5px;
}
</style>
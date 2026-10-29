<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'

interface FtpEntry {
  name: string
  size: number
  modified: string
  is_dir: boolean
}

const { t } = useI18n()
const host = ref('')
const port = ref(21)
const user = ref('')
const pass = ref('')
const passive = ref(true)
const connected = ref(false)
const cwd = ref('/')
const entries = ref<FtpEntry[]>([])
const busy = ref(false)
const msg = ref('')

function rpath(name: string) {
  return cwd.value.endsWith('/') ? cwd.value + name : cwd.value + '/' + name
}
function fmtSize(n: number) {
  if (n < 1024) return n + ' B'
  if (n < 1048576) return (n / 1024).toFixed(1) + ' KB'
  return (n / 1048576).toFixed(1) + ' MB'
}
async function doList(path: string) {
  entries.value = await invoke<FtpEntry[]>('ftp_list', { path })
  cwd.value = path
}
async function connect() {
  busy.value = true
  msg.value = ''
  try {
    await invoke('ftp_connect', { host: host.value, port: port.value, user: user.value, pass: pass.value, passive: passive.value })
    connected.value = true
    await doList('/')
  } catch (e) {
    msg.value = t('ftp.failed', { e: String(e) })
  } finally {
    busy.value = false
  }
}
async function disconnect() {
  await invoke('ftp_disconnect')
  connected.value = false
  entries.value = []
  msg.value = ''
}
async function enter(entry: FtpEntry) {
  if (!entry.is_dir) return
  busy.value = true
  try {
    await doList(rpath(entry.name))
  } catch (e) {
    msg.value = t('ftp.failed', { e: String(e) })
  } finally {
    busy.value = false
  }
}
async function up() {
  if (cwd.value === '/' || cwd.value === '') return
  const parts = cwd.value.replace(/\/+$/, '').split('/')
  parts.pop()
  const parent = parts.join('/') || '/'
  busy.value = true
  try {
    await doList(parent)
  } catch (e) {
    msg.value = t('ftp.failed', { e: String(e) })
  } finally {
    busy.value = false
  }
}
async function refresh() {
  busy.value = true
  try {
    await doList(cwd.value)
  } catch (e) {
    msg.value = t('ftp.failed', { e: String(e) })
  } finally {
    busy.value = false
  }
}
async function download(entry: FtpEntry) {
  if (entry.is_dir) return
  const local = await save({ defaultPath: entry.name })
  if (!local) return
  busy.value = true
  msg.value = ''
  try {
    await invoke('ftp_download', { remote: rpath(entry.name), local })
    msg.value = t('ftp.done', { p: local })
  } catch (e) {
    msg.value = t('ftp.failed', { e: String(e) })
  } finally {
    busy.value = false
  }
}
async function upload() {
  const sel = await open({ multiple: false })
  if (!sel) return
  const local = Array.isArray(sel) ? sel[0] : sel
  if (!local) return
  const name = local.split(/[/\\]/).pop() || 'file'
  busy.value = true
  msg.value = ''
  try {
    await invoke('ftp_upload', { local, remote: rpath(name) })
    await doList(cwd.value)
    msg.value = t('ftp.uploaded', { n: name })
  } catch (e) {
    msg.value = t('ftp.failed', { e: String(e) })
  } finally {
    busy.value = false
  }
}
async function removeEntry(entry: FtpEntry) {
  if (!confirm(t('ftp.confirmDelete', { n: entry.name }))) return
  busy.value = true
  try {
    await invoke('ftp_delete', { path: rpath(entry.name) })
    await doList(cwd.value)
  } catch (e) {
    msg.value = t('ftp.failed', { e: String(e) })
  } finally {
    busy.value = false
  }
}
async function renameEntry(entry: FtpEntry) {
  const nn = prompt(t('ftp.renamePrompt'), entry.name)
  if (!nn || nn === entry.name) return
  busy.value = true
  try {
    await invoke('ftp_rename', { from: rpath(entry.name), to: rpath(nn) })
    await doList(cwd.value)
  } catch (e) {
    msg.value = t('ftp.failed', { e: String(e) })
  } finally {
    busy.value = false
  }
}
async function newFolder() {
  const name = prompt(t('ftp.mkdirPrompt'), 'new-folder')
  if (!name) return
  busy.value = true
  try {
    await invoke('ftp_mkdir', { path: rpath(name) })
    await doList(cwd.value)
  } catch (e) {
    msg.value = t('ftp.failed', { e: String(e) })
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="ftp-tool">
    <!-- 连接表单 -->
    <div v-if="!connected" class="ftp-connect">
      <h1>{{ t('ftp.title') }}</h1>
      <div class="ftp-form">
        <label>{{ t('ftp.host') }}<input v-model="host" placeholder="ftp.example.com" /></label>
        <label>{{ t('ftp.port') }}<input v-model.number="port" type="number" /></label>
        <label>{{ t('ftp.user') }}<input v-model="user" /></label>
        <label>{{ t('ftp.pass') }}<input v-model="pass" type="password" /></label>
        <label class="ftp-passive"><input v-model="passive" type="checkbox" /> {{ t('ftp.passive') }}</label>
      </div>
      <button class="btn btn-primary" :disabled="busy || !host" @click="connect">
        {{ busy ? t('ftp.connecting') : t('ftp.connect') }}
      </button>
      <div v-if="msg" class="ftp-msg">⚠ {{ msg }}</div>
      <p class="ftp-note">{{ t('ftp.note') }}</p>
    </div>

    <!-- 文件浏览 -->
    <div v-else class="ftp-browse">
      <div class="ftp-bar">
        <button class="btn" :disabled="cwd === '/' || busy" @click="up">↑ {{ t('ftp.parent') }}</button>
        <span class="ftp-cwd">{{ cwd }}</span>
        <button class="btn" :disabled="busy" @click="refresh">{{ t('ftp.refresh') }}</button>
        <button class="btn" :disabled="busy" @click="newFolder">{{ t('ftp.mkdir') }}</button>
        <button class="btn btn-primary" :disabled="busy" @click="upload">{{ t('ftp.upload') }}</button>
        <button class="btn" @click="disconnect">{{ t('ftp.disconnect') }}</button>
      </div>
      <div v-if="msg" class="ftp-msg">{{ msg }}</div>
      <table class="ftp-list">
        <thead>
          <tr>
            <th>{{ t('ftp.name') }}</th>
            <th class="num">{{ t('ftp.size') }}</th>
            <th>{{ t('ftp.modified') }}</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="e in entries" :key="e.name" @dblclick="enter(e)">
            <td class="fname">{{ e.is_dir ? '📁' : '📄' }} {{ e.name }}</td>
            <td class="num">{{ e.is_dir ? '—' : fmtSize(e.size) }}</td>
            <td class="mtime">{{ e.modified }}</td>
            <td class="ftp-actions">
              <button v-if="!e.is_dir" class="btn btn-small" :disabled="busy" @click="download(e)">{{ t('ftp.download') }}</button>
              <button class="btn btn-small" :disabled="busy" @click="renameEntry(e)">{{ t('ftp.rename') }}</button>
              <button class="btn btn-small btn-danger" :disabled="busy" @click="removeEntry(e)">{{ t('ftp.delete') }}</button>
            </td>
          </tr>
          <tr v-if="!entries.length && !busy">
            <td colspan="4" class="ftp-empty">{{ t('ftp.empty') }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<style scoped>
.ftp-tool {
  height: 100%;
  overflow-y: auto;
  padding: 24px 28px;
}

.ftp-connect {
  max-width: 460px;
  margin: 40px auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.ftp-connect h1 {
  font-size: 22px;
  margin: 0;
  color: var(--text);
}

.ftp-form {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.ftp-form label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
  color: var(--text-muted);
}

.ftp-form input {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  color: var(--text);
  font-size: 13px;
}

.ftp-form input:focus {
  outline: none;
  border-color: var(--accent);
}

.ftp-note {
  font-size: 12px;
  color: var(--text-muted);
  margin: 0;
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 7px 14px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--surface);
  color: var(--text);
  font-size: 13px;
  cursor: pointer;
  white-space: nowrap;
}

.btn:hover {
  background: var(--surface-hover);
  border-color: var(--accent);
}

.btn:disabled {
  opacity: 0.45;
  cursor: default;
}

.btn-primary {
  background: var(--accent);
  color: var(--accent-fg);
  border-color: var(--accent);
}

.btn-primary:hover {
  background: var(--accent-hover);
}

.btn-small {
  padding: 3px 10px;
  font-size: 12px;
}

.ftp-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  flex-wrap: wrap;
}

.ftp-cwd {
  flex: 1;
  min-width: 120px;
  font-family: var(--font-mono);
  font-size: 13px;
  color: var(--text);
  padding: 6px 10px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ftp-msg {
  font-size: 13px;
  color: var(--accent);
  margin-bottom: 10px;
  word-break: break-all;
}

.ftp-list {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.ftp-list th,
.ftp-list td {
  text-align: left;
  padding: 8px 10px;
  border-bottom: 1px solid var(--border);
}

.ftp-list th {
  color: var(--text-muted);
  font-weight: 500;
  font-size: 12px;
}

.ftp-list tbody tr:hover {
  background: var(--surface);
  cursor: default;
}

.ftp-list .num {
  text-align: right;
  font-family: var(--font-mono);
}

.fname {
  font-family: var(--font-mono);
  color: var(--text);
}

.mtime {
  color: var(--text-muted);
}

.ftp-empty {
  text-align: center;
  color: var(--text-muted);
  padding: 24px;
}

.ftp-passive {
  flex-direction: row;
  align-items: center;
  gap: 6px;
}

.ftp-actions {
  display: flex;
  gap: 4px;
  justify-content: flex-end;
}

.btn-danger:hover {
  background: var(--error);
  border-color: var(--error);
  color: #fff;
}
</style>
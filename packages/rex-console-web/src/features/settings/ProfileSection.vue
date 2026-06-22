<template>
  <div class="settings-section">
    <div class="section-header">
      <h3 class="section-title">👤 {{ t('settings.profile.title') }}</h3>
    </div>

    <!-- 用户名 -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">{{ t('settings.profile.username') }}</div>
      </div>
      <button class="btn btn-ghost btn-sm" @click="showEditUsername = true">{{ t('settings.profile.edit') }}</button>
    </div>

    <!-- 密码 -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">{{ t('settings.profile.password') }}</div>
        <div class="settings-row-desc">{{ t('settings.profile.lastChanged') }}</div>
      </div>
      <button class="btn btn-ghost btn-sm" @click="showChangePassword = true">{{ t('settings.profile.change') }}</button>
    </div>

    <!-- 编辑用户名弹窗 -->
    <div v-if="showEditUsername" class="modal-overlay" @click.self="showEditUsername = false">
      <div class="modal">
        <div class="modal-title">{{ t('settings.profile.editTitle') }}</div>
        <div class="form-group">
          <label class="form-label">{{ t('settings.profile.username') }}</label>
          <input
            v-model="newUsername"
            class="form-input"
            :placeholder="t('settings.profile.usernamePlaceholder')"
            @keydown.enter="saveUsername"
          />
        </div>
        <div class="modal-actions">
          <button class="btn" @click="showEditUsername = false">{{ t('settings.profile.cancel') }}</button>
          <button class="btn btn-primary" @click="saveUsername" :disabled="!newUsername.trim()">{{ t('settings.profile.save') }}</button>
        </div>
      </div>
    </div>

    <!-- 修改密码弹窗 -->
    <div v-if="showChangePassword" class="modal-overlay" @click.self="showChangePassword = false">
      <div class="modal">
        <div class="modal-title">{{ t('settings.profile.changeTitle') }}</div>
        <div class="form-group">
          <label class="form-label">{{ t('settings.profile.currentPassword') }}</label>
          <input
            v-model="currentPassword"
            type="password"
            class="form-input"
            :placeholder="t('settings.profile.currentPasswordPlaceholder')"
          />
        </div>
        <div class="form-group" style="margin-top: var(--sp-md)">
          <label class="form-label">{{ t('settings.profile.newPassword') }}</label>
          <input
            v-model="newPassword"
            type="password"
            class="form-input"
            :placeholder="t('settings.profile.newPasswordPlaceholder')"
            @keydown.enter="savePassword"
          />
        </div>
        <div class="modal-actions">
          <button class="btn" @click="showChangePassword = false">{{ t('settings.profile.cancel') }}</button>
          <button
            class="btn btn-primary"
            @click="savePassword"
            :disabled="!currentPassword || newPassword.length < 6"
          >{{ t('settings.profile.save') }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { getUserProfile, updateUserProfile, changePassword } from '@/api/settings'

const { t } = useI18n()

const username = ref('admin')
const showEditUsername = ref(false)
const newUsername = ref('')
const showChangePassword = ref(false)
const currentPassword = ref('')
const newPassword = ref('')

async function loadProfile() {
  try {
    const profile = await getUserProfile()
    username.value = profile.username
  } catch {
    // ignore
  }
}

async function saveUsername() {
  if (!newUsername.value.trim()) return
  try {
    const profile = await updateUserProfile(newUsername.value.trim())
    username.value = profile.username
    showEditUsername.value = false
  } catch (err: any) {
    alert(err?.response?.data?.error?.message ?? t('settings.profile.saveFailed'))
  }
}

async function savePassword() {
  if (!currentPassword.value || newPassword.value.length < 6) return
  try {
    await changePassword({
      current_password: currentPassword.value,
      new_password: newPassword.value,
    })
    showChangePassword.value = false
    currentPassword.value = ''
    newPassword.value = ''
    alert(t('settings.profile.saveSuccess'))
  } catch (err: any) {
    alert(err?.response?.data?.error?.message ?? t('settings.profile.passwordFailed'))
  }
}

onMounted(loadProfile)
</script>

<style scoped>
.settings-section {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--sp-xl);
}

.section-header {
  margin-bottom: var(--sp-lg);
}

.section-title {
  font-family: var(--font-mono);
  font-size: var(--fs-md);
  font-weight: 600;
}

.settings-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-md) 0;
  border-bottom: 1px solid var(--border);
}

.settings-row:last-child {
  border-bottom: none;
}

.settings-row-info {
  flex: 1;
}

.settings-row-label {
  font-size: var(--fs-sm);
  font-weight: 500;
  color: var(--text-primary);
}

.settings-row-desc {
  font-size: var(--fs-xs);
  color: var(--text-secondary);
  margin-top: 2px;
}

.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: var(--z-modal);
}

.modal {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--sp-xl);
  max-width: 400px;
  width: 90%;
}

.modal-title {
  font-size: var(--fs-md);
  font-weight: 600;
  margin-bottom: var(--sp-lg);
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-sm);
  margin-top: var(--sp-lg);
}
</style>

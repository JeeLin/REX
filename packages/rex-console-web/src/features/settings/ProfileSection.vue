<template>
  <div class="settings-section">
    <div class="section-header">
      <h3 class="section-title">👤 个人信息</h3>
    </div>

    <!-- 用户名 -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">用户名</div>
      </div>
      <button class="btn btn-ghost btn-sm" @click="showEditUsername = true">编辑</button>
    </div>

    <!-- 密码 -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">密码</div>
        <div class="settings-row-desc">上次修改时间未知</div>
      </div>
      <button class="btn btn-ghost btn-sm" @click="showChangePassword = true">修改</button>
    </div>

    <!-- 编辑用户名弹窗 -->
    <div v-if="showEditUsername" class="modal-overlay" @click.self="showEditUsername = false">
      <div class="modal">
        <div class="modal-title">编辑用户名</div>
        <div class="form-group">
          <label class="form-label">用户名</label>
          <input
            v-model="newUsername"
            class="form-input"
            placeholder="输入新用户名"
            @keydown.enter="saveUsername"
          />
        </div>
        <div class="modal-actions">
          <button class="btn" @click="showEditUsername = false">取消</button>
          <button class="btn btn-primary" @click="saveUsername" :disabled="!newUsername.trim()">保存</button>
        </div>
      </div>
    </div>

    <!-- 修改密码弹窗 -->
    <div v-if="showChangePassword" class="modal-overlay" @click.self="showChangePassword = false">
      <div class="modal">
        <div class="modal-title">修改密码</div>
        <div class="form-group">
          <label class="form-label">当前密码</label>
          <input
            v-model="currentPassword"
            type="password"
            class="form-input"
            placeholder="输入当前密码"
          />
        </div>
        <div class="form-group" style="margin-top: var(--sp-md)">
          <label class="form-label">新密码</label>
          <input
            v-model="newPassword"
            type="password"
            class="form-input"
            placeholder="输入新密码（至少 6 位）"
            @keydown.enter="savePassword"
          />
        </div>
        <div class="modal-actions">
          <button class="btn" @click="showChangePassword = false">取消</button>
          <button
            class="btn btn-primary"
            @click="savePassword"
            :disabled="!currentPassword || newPassword.length < 6"
          >保存</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getUserProfile, updateUserProfile, changePassword } from '@/api/settings'

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
    alert(err?.response?.data?.error?.message ?? '保存失败')
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
    alert('密码修改成功')
  } catch (err: any) {
    alert(err?.response?.data?.error?.message ?? '密码修改失败')
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

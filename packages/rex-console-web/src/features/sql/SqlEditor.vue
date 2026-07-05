<template>
  <div class="sql-editor-wrap">
    <SqlCodeMirror
      ref="cmRef"
      :modelValue="modelValue"
      :dialect="dialect"
      @update:modelValue="$emit('update:modelValue', $event)"
      @execute="$emit('execute')"
      @save="$emit('save')"
      @contextmenu.prevent="handleContextMenu"
    />
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useContextMenu } from '@/composables/useContextMenu'
import SqlCodeMirror from './SqlCodeMirror.vue'

const { t } = useI18n()
const { show: showMenu } = useContextMenu()

defineProps<{
  modelValue: string
  dialect?: 'mysql' | 'postgresql' | 'sqlite' | 'sql'
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  'execute': []
  'executeSelection': [sql: string]
  'save': []
  'showHistory': []
  'format': []
}>()

const cmRef = ref<InstanceType<typeof SqlCodeMirror>>()

function handleContextMenu(event: MouseEvent) {
  const selection = cmRef.value?.getSelection() ?? ''
  showMenu(event, [
    {
      label: t('sql.ctx.executeSelection'),
      action: () => { emit('executeSelection', selection) },
      disabled: !selection,
    },
    {
      label: t('sql.ctx.executeAll'),
      action: () => { emit('execute') },
    },
    { separator: true },
    {
      label: t('sql.ctx.cut'),
      action: () => {
        navigator.clipboard.writeText(selection)
        cmRef.value?.replaceSelection('')
      },
      disabled: !selection,
    },
    {
      label: t('sql.ctx.copy'),
      action: () => { navigator.clipboard.writeText(selection) },
      disabled: !selection,
    },
    {
      label: t('sql.ctx.paste'),
      action: async () => {
        const text = await navigator.clipboard.readText()
        cmRef.value?.replaceSelection(text)
      },
    },
    { separator: true },
    {
      label: t('sql.ctx.format'),
      action: () => { cmRef.value?.formatSql() },
    },
    {
      label: t('sql.ctx.caseConvert'),
      children: [
        { label: t('sql.ctx.caseUpper'), action: () => cmRef.value?.convertCase('upper') },
        { label: t('sql.ctx.caseLower'), action: () => cmRef.value?.convertCase('lower') },
        { label: t('sql.ctx.caseTitle'), action: () => cmRef.value?.convertCase('title') },
      ],
    },
    {
      label: t('sql.ctx.toggleComment'),
      action: () => { cmRef.value?.toggleComment() },
    },
    { separator: true },
    {
      label: t('sql.ctx.save'),
      action: () => { emit('save') },
    },
    {
      label: t('sql.ctx.insertTemplate'),
      children: [
        { label: 'SELECT', action: () => cmRef.value?.insertText('SELECT * FROM  WHERE  LIMIT 100;') },
        { label: 'INSERT', action: () => cmRef.value?.insertText('INSERT INTO  () VALUES ();') },
        { label: 'UPDATE', action: () => cmRef.value?.insertText('UPDATE  SET  WHERE ;') },
        { label: 'DELETE', action: () => cmRef.value?.insertText('DELETE FROM  WHERE ;') },
        { label: 'CREATE TABLE', action: () => cmRef.value?.insertText('CREATE TABLE  (\n  id INT PRIMARY KEY AUTO_INCREMENT,\n  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP\n);') },
      ],
    },
    {
      label: t('sql.ctx.history'),
      action: () => { emit('showHistory') },
    },
  ])
}

function formatSql() {
  cmRef.value?.formatSql()
}

defineExpose({ formatSql })
</script>

<style scoped>
.sql-editor-wrap {
  flex: 1;
  min-height: 120px;
  position: relative;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
</style>

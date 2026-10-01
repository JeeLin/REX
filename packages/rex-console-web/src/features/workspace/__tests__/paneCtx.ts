import type { Component, DefineComponent } from 'vue'
import { computed, ref } from 'vue'
import { mount, type VueWrapper } from '@vue/test-utils'
import type { MountingOptions } from '@vue/test-utils'
import type { Tab } from '@/composables/useTabs'
import { PANE_CTX, type PaneCtx } from '@/features/workspace/paneContext'

type Stubs = NonNullable<MountingOptions<Record<string, unknown>>['global']>['stubs']

// PaneCtx stub with only the members page-level shortcut ownership reads.
export function buildPaneCtx(
  leaves: { id: string; tabId: string | null }[],
  activePaneId: string,
  activeTabId: string,
  tab: Pick<Tab, 'label' | 'protocol'>,
): PaneCtx {
  return {
    allLeaves: ref(leaves),
    activePaneId: ref(activePaneId),
    activeTabInfo: computed(() => ({ id: activeTabId, ...tab, status: 'connected' })),
  } as unknown as PaneCtx
}

// Mounts one page instance inside a pane context; the caller owns cleanup.
export function mountInPane(
  component: Component,
  ctx: PaneCtx,
  props: Record<string, unknown>,
  stubs?: Stubs,
): VueWrapper {
  const wrapper = mount(component as DefineComponent, {
    props,
    global: {
      provide: { [PANE_CTX]: ctx },
      ...(stubs ? { stubs } : {}),
    },
    attachTo: document.body,
  })
  return wrapper
}

// REX Hub Shared Module — Sidebar, Theme, i18n, Context Menu

// ─── i18n ───
const I18N = {
  zh: {
    // 导航
    workspace: '工作区', dashboard: '仪表盘', environments: '环境', agents: 'Agent', settings: '设置', auditLog: '审计日志',
    // 侧边栏
    search: '搜索资源...', favorites: '收藏', recent: '最近使用', nav: '导航', envs: '环境',
    noFavorites: '暂无收藏', noRecent: '暂无记录', newEnv: '+ 新建环境',
    allEnvs: '所有环境', allResources: '全部资源',
    // 状态
    online: '在线', offline: '离线', connecting: '连接中',
    // 操作
    connect: '连接', add: '+ 添加资源', addResource: '添加资源',
    // 工作空间
    newConnection: '新建连接', closeTab: '关闭', closeOthers: '关闭其他',
    closeRight: '关闭右侧', closeLeft: '关闭左侧', closeAll: '关闭全部',
    duplicate: '复制标签', moveToPanel: '移动到面板', allDisconnect: '全部断开',
    noSessions: '没有打开的会话',
    workspaceHint: '从侧边栏选择资源开始连接\n按 Ctrl+N 新建连接 · Alt+1~5 切换布局 · F1 快捷键',
    // 终端
    copy: '复制', paste: '粘贴', clear: '清屏', selectAll: '全选',
    reconnect: '重连', disconnect: '断开连接', openSftp: '打开 SFTP 面板',
    openSftpNewTab: '在新标签中打开 SFTP', copyAddr: '复制连接地址',
    // SQL
    execute: '执行', executeSelected: '执行选中 SQL', executeAll: '执行全部 SQL',
    format: '格式化', save: '保存', saveAs: '另存为…', rename: '重命名',
    copySql: '复制 SQL', executeSql: '执行 SQL', history: '历史记录',
    insertTemplate: '插入模板', comment: '注释/取消注释',
    caseConvert: '大小写转换', upperCase: '全部大写', lowerCase: '全部小写', titleCase: '首字母大写',
    copyRow: '复制行', copyCell: '复制单元格', copyColumn: '复制整列', copyAsJson: '复制为 JSON',
    sortAsc: '按此列升序排列', sortDesc: '按此列降序排列',
    exportRow: '导出当前行', generateUpdate: '在编辑器中生成 UPDATE', generateDelete: '在编辑器中生成 DELETE',
    viewStructure: '查看表结构', viewCount: '查看行数统计', copyTableName: '复制表名',
    selectStar: 'SELECT * 查询', exportTable: '导出表数据', refresh: '刷新',
    copyColumnName: '复制列名', copyColumnType: '复制列类型', viewConstraints: '查看列约束',
    newTable: '新建表', copyDbName: '复制数据库名',
    viewDefinition: '查看定义', copyViewName: '复制视图名',
    expandAll: '全部展开', collapseAll: '全部折叠',
    closeSaved: '关闭已保存的', copyToClipboard: '复制到剪贴板',
    // SFTP
    open: '打开', download: '下载', copyPath: '复制路径', copyFileName: '复制文件名',
    deleteFile: '删除', properties: '属性',
    uploadFile: '上传文件', uploadFolder: '上传文件夹', newFile: '新建文件', newFolder: '新建文件夹',
    paste: '粘贴', selectAll: '全选',
    sendTo: '发送到…', transferComplete: '传输完成',
    // Agent
    viewLog: '查看日志', config: '配置', copyAgentId: '复制 Agent ID',
    copyToken: '复制注册令牌', restart: '重启 Agent', resetToken: '重置令牌',
    // 环境
    openDetail: '打开详情', openAllInWorkspace: '在工作区打开',
    newResource: '新建资源', addAgent: '添加 Agent', editEnv: '编辑环境', deleteEnv: '删除环境',
    editResource: '编辑资源', deleteResource: '删除资源',
    // 审计日志
    viewDetail: '查看详情', copySummary: '复制摘要', copyOpType: '复制操作类型',
    copyTimestamp: '复制时间戳', filterByOp: '按此操作类型筛选', filterByEnv: '按此环境筛选',
    exportAll: '导出全部为 CSV', clearFilters: '清除筛选条件',
    // 侧边栏
    cancelFavorite: '取消收藏', removeFromRecent: '从最近使用中移除', clearRecent: '清空最近使用',
    collapseExpand: '折叠/展开',
    // 仪表盘
    refreshStats: '刷新统计',
    // 仪表盘页面
    envCount: '环境', resCount: '资源', agentOnline: 'Agent 在线', todayOps: '今日操作',
    quickConnect: '快速连接', recentUsed: '最近使用',
    // 环境详情
    directConnect: '直连', agentConnect: 'Agent 代理',
    resources: '个资源', lastActive: '最后活跃',
    // Agent 页面
    setupGuide: '安装指南', agentSetupDesc: '在内网服务器上安装并注册 Agent',
    downloadAgent: '下载 Agent', registerAgent: '注册 Agent',
    agentLogs: 'Agent 日志', agentConfig: 'Agent 配置',
    copySuccess: '已复制到剪贴板',
    // 设置页面
    personalInfo: '个人信息', username: '用户名', password: '密码',
    lastModified: '上次修改', edit: '编辑', changePwd: '修改',
    appearance: '外观', selectTheme: '选择界面显示主题',
    darkTheme: '深色', lightTheme: '浅色', autoTheme: '跟随系统',
    selectLang: '选择界面语言', sidebarToggle: '侧边栏',
    sidebarDesc: '桌面端固定或可折叠侧边栏',
    termSettings: '终端', fontSize: '字体大小', fontFamily: '字体族',
    termFontDesc: 'SSH 终端默认字体大小', termFontFamDesc: '终端使用的等宽字体',
    cursorBlink: '光标闪烁', cursorBlinkDesc: '终端光标闪烁动画',
    keepalive: '保活间隔', keepaliveDesc: 'SSH 连接保活探针间隔（秒）',
    security: '安全', sessionTimeout: '会话超时',
    // 审计日志
    timeRange: '时间', last1h: '最近 1 小时', last24h: '最近 24 小时',
    last7d: '最近 7 天', last30d: '最近 30 天',
    allUsers: '全部', allEnvs: '全部', exportCsv: '导出 CSV',
    // 通用
    confirm: '确认', cancel: '取消', close: '关闭', openInWorkspace: '在工作区打开',
    copyDelayInfo: '复制延迟信息', openConnectionDetail: '打开连接详情', toggleFullscreen: '切换全屏',
    sendToTarget: '发送到目标', allEnvironments: '所有环境',
  },
  en: {
    // Navigation
    workspace: 'Workspace', dashboard: 'Dashboard', environments: 'Environments', agents: 'Agents', settings: 'Settings', auditLog: 'Audit Log',
    // Sidebar
    search: 'Search resources...', favorites: 'Favorites', recent: 'Recent', nav: 'Navigation', envs: 'Environments',
    noFavorites: 'No favorites yet', noRecent: 'No recent items', newEnv: '+ New Environment',
    allEnvs: 'All Environments', allResources: 'All Resources',
    // Status
    online: 'Online', offline: 'Offline', connecting: 'Connecting',
    // Actions
    connect: 'Connect', add: '+ Add Resource', addResource: 'Add Resource',
    // Workspace
    newConnection: 'New Connection', closeTab: 'Close', closeOthers: 'Close Others',
    closeRight: 'Close Right', closeLeft: 'Close Left', closeAll: 'Close All',
    duplicate: 'Duplicate Tab', moveToPanel: 'Move to Panel', allDisconnect: 'Disconnect All',
    noSessions: 'No open sessions',
    workspaceHint: 'Select a resource from the sidebar to connect\nPress Ctrl+N for new connection · Alt+1~5 to switch layout · F1 for shortcuts',
    // Terminal
    copy: 'Copy', paste: 'Paste', clear: 'Clear', selectAll: 'Select All',
    reconnect: 'Reconnect', disconnect: 'Disconnect', openSftp: 'Open SFTP Panel',
    openSftpNewTab: 'Open SFTP in New Tab', copyAddr: 'Copy Connection Address',
    // SQL
    execute: 'Execute', executeSelected: 'Execute Selected SQL', executeAll: 'Execute All SQL',
    format: 'Format', save: 'Save', saveAs: 'Save As...', rename: 'Rename',
    copySql: 'Copy SQL', executeSql: 'Execute SQL', history: 'History',
    insertTemplate: 'Insert Template', comment: 'Toggle Comment',
    caseConvert: 'Case Convert', upperCase: 'UPPERCASE', lowerCase: 'lowercase', titleCase: 'Title Case',
    copyRow: 'Copy Row', copyCell: 'Copy Cell', copyColumn: 'Copy Column', copyAsJson: 'Copy as JSON',
    sortAsc: 'Sort Ascending', sortDesc: 'Sort Descending',
    exportRow: 'Export Current Row', generateUpdate: 'Generate UPDATE', generateDelete: 'Generate DELETE',
    viewStructure: 'View Structure', viewCount: 'View Row Count', copyTableName: 'Copy Table Name',
    selectStar: 'SELECT * Query', exportTable: 'Export Table Data', refresh: 'Refresh',
    copyColumnName: 'Copy Column Name', copyColumnType: 'Copy Column Type', viewConstraints: 'View Constraints',
    newTable: 'New Table', copyDbName: 'Copy Database Name',
    viewDefinition: 'View Definition', copyViewName: 'Copy View Name',
    expandAll: 'Expand All', collapseAll: 'Collapse All',
    closeSaved: 'Close Saved', copyToClipboard: 'Copy to Clipboard',
    // SFTP
    open: 'Open', download: 'Download', copyPath: 'Copy Path', copyFileName: 'Copy File Name',
    deleteFile: 'Delete', properties: 'Properties',
    uploadFile: 'Upload File', uploadFolder: 'Upload Folder', newFile: 'New File', newFolder: 'New Folder',
    paste: 'Paste', selectAll: 'Select All',
    sendTo: 'Send To...', transferComplete: 'Transfer Complete',
    // Agent
    viewLog: 'View Log', config: 'Configure', copyAgentId: 'Copy Agent ID',
    copyToken: 'Copy Registration Token', restart: 'Restart Agent', resetToken: 'Reset Token',
    // Environment
    openDetail: 'Open Detail', openAllInWorkspace: 'Open All in Workspace',
    newResource: 'New Resource', addAgent: 'Add Agent', editEnv: 'Edit Environment', deleteEnv: 'Delete Environment',
    editResource: 'Edit Resource', deleteResource: 'Delete Resource',
    // Audit Log
    viewDetail: 'View Details', copySummary: 'Copy Summary', copyOpType: 'Copy Operation Type',
    copyTimestamp: 'Copy Timestamp', filterByOp: 'Filter by Operation', filterByEnv: 'Filter by Environment',
    exportAll: 'Export All as CSV', clearFilters: 'Clear Filters',
    // Sidebar
    cancelFavorite: 'Remove from Favorites', removeFromRecent: 'Remove from Recent', clearRecent: 'Clear Recent',
    collapseExpand: 'Collapse/Expand',
    // Dashboard
    refreshStats: 'Refresh Stats',
    // Dashboard page
    envCount: 'Environments', resCount: 'Resources', agentOnline: 'Agents Online', todayOps: 'Today\'s Ops',
    quickConnect: 'Quick Connect', recentUsed: 'Recently Used',
    // Environment detail
    directConnect: 'Direct', agentConnect: 'Agent Proxy',
    resources: 'resources', lastActive: 'Last active',
    // Agent page
    setupGuide: 'Setup Guide', agentSetupDesc: 'Install and register Agent on intranet servers',
    downloadAgent: 'Download Agent', registerAgent: 'Register Agent',
    agentLogs: 'Agent Logs', agentConfig: 'Agent Config',
    copySuccess: 'Copied to clipboard',
    // Settings page
    personalInfo: 'Personal Info', username: 'Username', password: 'Password',
    lastModified: 'Last modified', edit: 'Edit', changePwd: 'Change',
    appearance: 'Appearance', selectTheme: 'Select interface theme',
    darkTheme: 'Dark', lightTheme: 'Light', autoTheme: 'System',
    selectLang: 'Select interface language', sidebarToggle: 'Sidebar',
    sidebarDesc: 'Fixed or collapsible sidebar on desktop',
    termSettings: 'Terminal', fontSize: 'Font Size', fontFamily: 'Font Family',
    termFontDesc: 'Default SSH terminal font size', termFontFamDesc: 'Monospace font for terminal',
    cursorBlink: 'Cursor Blink', cursorBlinkDesc: 'Terminal cursor blink animation',
    keepalive: 'Keepalive', keepaliveDesc: 'SSH keepalive interval (seconds)',
    security: 'Security', sessionTimeout: 'Session Timeout',
    // Audit Log
    timeRange: 'Time', last1h: 'Last 1 hour', last24h: 'Last 24 hours',
    last7d: 'Last 7 days', last30d: 'Last 30 days',
    allUsers: 'All', allEnvs: 'All', exportCsv: 'Export CSV',
    // Common
    confirm: 'Confirm', cancel: 'Cancel', close: 'Close', openInWorkspace: 'Open in Workspace',
    copyDelayInfo: 'Copy Delay Info', openConnectionDetail: 'Open Connection Detail', toggleFullscreen: 'Toggle Fullscreen',
    sendToTarget: 'Send to Target', allEnvironments: 'All Environments',
  }
};

let currentLang = localStorage.getItem('rex-lang') || 'zh';
function t(key) { return I18N[currentLang][key] || key; }

// ─── Theme ───
let currentTheme = localStorage.getItem('rex-theme') || 'dark';
const systemDarkQuery = window.matchMedia('(prefers-color-scheme: dark)');

function getEffectiveTheme(theme) {
  if (theme === 'auto') return systemDarkQuery.matches ? 'dark' : 'light';
  return theme;
}

function applyTheme(theme) {
  currentTheme = theme;
  localStorage.setItem('rex-theme', theme);
  document.documentElement.setAttribute('data-theme', getEffectiveTheme(theme));
}

// Listen for system theme changes (auto mode)
const onSystemThemeChange = () => {
  if (currentTheme === 'auto') {
    document.documentElement.setAttribute('data-theme', getEffectiveTheme('auto'));
  }
};
if (systemDarkQuery.addEventListener) {
  systemDarkQuery.addEventListener('change', onSystemThemeChange);
} else if (systemDarkQuery.addListener) {
  systemDarkQuery.addListener(onSystemThemeChange);
}

// Apply on load
applyTheme(currentTheme);

// ─── Sidebar Collapse State ───
const savedCollapse = localStorage.getItem('rex-sidebar-collapsed');
if (savedCollapse === '1') {
  document.addEventListener('DOMContentLoaded', () => {
    const sidebar = document.getElementById('appSidebar');
    if (sidebar) {
      sidebar.classList.add('collapsed');
      const mainContent = document.querySelector('.main-content, .ws-main');
      if (mainContent) mainContent.style.marginLeft = '48px';
    }
  });
}

// ─── Favorites & Recent ───
const DEFAULT_FAVORITES = [
  { name: '云服务器', protocol: 'ssh', addr: 'root@192.168.1.100:22', href: 'app.html' },
  { name: '主数据库', protocol: 'mysql', addr: 'db.internal:3306', href: 'app.html' },
  { name: 'Web 文件服务器', protocol: 'sftp', addr: 'root@192.168.1.100', href: 'app.html' },
];
const DEFAULT_RECENT = [
  { name: '云服务器', protocol: 'ssh', addr: 'root@192.168.1.100:22', href: 'app.html', time: '5分钟前' },
  { name: '主数据库', protocol: 'mysql', addr: 'db.internal:3306', href: 'app.html', time: '1小时前' },
  { name: 'Web 文件服务器', protocol: 'sftp', addr: 'root@192.168.1.100', href: 'app.html', time: '昨天' },
  { name: '分析数据库', protocol: 'postgresql', addr: 'analytics.internal:5432', href: 'app.html', time: '2天前' },
];

const PROTOCOL_ICONS = {
  ssh: '$', sftp: '📁', mysql: 'dB', postgresql: 'pg',
  redis: 'R', docker: '🐳', sqlite: 'S', s3: '☁',
};
const PROTOCOL_COLORS = {
  ssh: 'var(--success)', sftp: 'var(--accent-purple)', mysql: 'var(--info)',
  postgresql: 'var(--accent-purple)', redis: 'var(--danger)', docker: 'var(--info)',
  sqlite: 'var(--warning)', s3: 'var(--accent)',
};

// ─── Environment data with resources ───
const ENVIRONMENTS = [
  {
    name: '阿里云', online: true, href: 'environment.html',
    resources: [
      { name: '云服务器', protocol: 'ssh', addr: '192.168.1.100:22', href: 'app.html' },
      { name: '主数据库', protocol: 'mysql', addr: 'db.internal:3306', href: 'app.html' },
      { name: '分析数据库', protocol: 'postgresql', addr: 'analytics.internal:5432', href: 'app.html' },
      { name: 'Web 文件服务器', protocol: 'sftp', addr: '192.168.1.100', href: 'app.html' },
      { name: 'Docker 服务', protocol: 'docker', addr: '192.168.1.100', href: 'app.html' },
      { name: '对象存储', protocol: 's3', addr: 'oss-cn-hangzhou', href: 'app.html' },
    ]
  },
  {
    name: '树莓派集群', online: true, href: 'environment.html',
    resources: [
      { name: '开发服务器', protocol: 'ssh', addr: '10.0.1.15:22', href: 'app.html' },
      { name: '测试数据库', protocol: 'mysql', addr: '10.0.1.20:3306', href: 'app.html' },
      { name: '缓存服务', protocol: 'redis', addr: '10.0.1.25:6379', href: 'app.html' },
      { name: '嵌入式数据库', protocol: 'sqlite', addr: '/opt/data/app.db', href: 'app.html' },
    ]
  },
  {
    name: '家庭 NAS', online: false, href: 'environment.html',
    resources: [
      { name: 'NAS 主机', protocol: 'ssh', addr: '192.168.0.100:22', href: 'app.html' },
      { name: 'NAS 文件', protocol: 'sftp', addr: '192.168.0.100', href: 'app.html' },
    ]
  },
];

// ─── Context Menu System ───
function hideAllContextMenus() {
  document.querySelectorAll('.ctx-menu').forEach(m => m.remove());
}
function showContextMenu(e, items) {
  e.preventDefault();
  hideAllContextMenus();
  const menu = document.createElement('div');
  menu.className = 'ctx-menu';
  items.forEach(item => {
    if (item === '---') {
      menu.appendChild(Object.assign(document.createElement('div'), { className: 'ctx-divider' }));
    } else {
      const el = document.createElement('div');
      el.className = 'ctx-item' + (item.danger ? ' ctx-danger' : '') + (item.disabled ? ' ctx-disabled' : '');
      el.innerHTML = (item.icon ? `<span class="ctx-icon">${item.icon}</span>` : '') +
        `<span>${item.label}</span>` +
        (item.shortcut ? `<span class="ctx-shortcut">${item.shortcut}</span>` : '') +
        (item.sub ? `<span class="ctx-arrow">▸</span>` : '');
      if (item.sub) {
        const subMenu = document.createElement('div');
        subMenu.className = 'ctx-menu ctx-submenu';
        item.sub.forEach(subItem => {
          const subEl = document.createElement('div');
          subEl.className = 'ctx-item';
          subEl.textContent = subItem.label;
          subEl.onclick = (ev) => { ev.stopPropagation(); hideAllContextMenus(); subItem.action?.(); };
          subMenu.appendChild(subEl);
        });
        el.appendChild(subMenu);
      }
      if (!item.disabled && item.action) {
        el.onclick = (ev) => { ev.stopPropagation(); hideAllContextMenus(); item.action(); };
      }
      menu.appendChild(el);
    }
  });
  document.body.appendChild(menu);
  // Position
  const x = Math.min(e.clientX, window.innerWidth - 220);
  const y = Math.min(e.clientY, window.innerHeight - menu.offsetHeight - 10);
  menu.style.left = x + 'px';
  menu.style.top = y + 'px';
}
document.addEventListener('click', hideAllContextMenus);

// ─── Sidebar HTML Generator ───
function buildSidebar(activePage) {
  const favNames = new Set(DEFAULT_FAVORITES.map(f => f.name));
  const quickItems = [
    ...DEFAULT_FAVORITES.map(f => ({ ...f, isFav: true })),
    ...DEFAULT_RECENT.filter(r => !favNames.has(r.name)).map(r => ({ ...r, isFav: false })),
  ];

  let envItems = ENVIRONMENTS.map((env, i) => {
    let resItems = env.resources.map(res =>
      `<a href="${res.href}" class="sb-resource-item" data-resource="${res.name}" data-proto="${res.protocol}">
        <span class="sb-resource-dot" style="background:${PROTOCOL_COLORS[res.protocol]}"></span>
        <span class="sb-resource-name">${res.name}</span>
        <span class="sb-resource-proto">${res.protocol.toUpperCase()}</span>
      </a>`
    ).join('');
    return `
      <div class="sb-env-group">
        <div class="sb-env-header" onclick="toggleEnvGroup(${i})">
          <span class="status-dot ${env.online ? 'connected' : 'disconnected'}"></span>
          <span class="sb-env-name">${env.name}</span>
          <span class="sb-env-count">${env.resources.length}</span>
          <span class="sb-env-chevron" id="chevron-${i}">▾</span>
        </div>
        <div class="sb-env-resources" id="env-resources-${i}">
          ${resItems}
        </div>
      </div>`;
  }).join('');

  return `
    <aside class="sidebar" id="appSidebar">
      <div class="sidebar-header">
        <div class="sidebar-logo">R</div>
        <span class="sidebar-brand">REX Hub</span>
        <div class="sidebar-header-actions">
          <button class="sidebar-icon-btn" onclick="toggleTheme()" title="Toggle Theme" id="themeBtn">${{dark:'☀',light:'☾',auto:'◐'}[currentTheme] || '☀'}</button>
          <button class="sidebar-icon-btn" onclick="toggleLang()" title="Switch Language" id="langBtn">${currentLang === 'zh' ? 'EN' : '中'}</button>
        </div>
      </div>

      <nav class="sidebar-nav">
        <div class="sb-search">
          <span class="sb-search-icon">⌕</span>
          <input type="text" placeholder="${t('search')}" class="sb-search-input">
        </div>

        <div class="nav-section">
          <a href="app.html" class="nav-item ${activePage === 'workspace' ? 'active' : ''}">
            <span class="nav-icon">⊞</span> <span>${t('workspace')}</span>
          </a>
          <a href="dashboard.html" class="nav-item ${activePage === 'dashboard' ? 'active' : ''}">
            <span class="nav-icon">◉</span> <span>${t('dashboard')}</span>
          </a>
          <a href="environments.html" class="nav-item ${activePage === 'environments' ? 'active' : ''}">
            <span class="nav-icon">◈</span> <span>${t('environments')}</span>
          </a>
          <a href="agents.html" class="nav-item ${activePage === 'agents' ? 'active' : ''}">
            <span class="nav-icon">⬡</span> <span>${t('agents')}</span>
          </a>
          ${(localStorage.getItem('rex-audit-enabled') !== 'false') ? `
          <a href="audit-log.html" class="nav-item ${activePage === 'audit-log' ? 'active' : ''}">
            <span class="nav-icon">📋</span> <span>${t('auditLog')}</span>
          </a>` : ''}
        </div>

        <div class="sb-section">
          <div class="sb-section-header">
            <span class="sb-section-title">${t('envs')}</span>
          </div>
          ${envItems}
        </div>

        <div class="sb-section">
          <div class="sb-section-header">
            <span class="sb-section-title">${t('favorites')}</span>
          </div>
          ${DEFAULT_FAVORITES.map(f => `
            <a href="${f.href}" class="sb-quick-item" data-proto="${f.protocol}">
              <span class="sb-quick-icon" style="background:${PROTOCOL_COLORS[f.protocol]}15;color:${PROTOCOL_COLORS[f.protocol]}">${PROTOCOL_ICONS[f.protocol] || '·'}</span>
              <span class="sb-quick-info">
                <span class="sb-quick-name">${f.name}</span>
                <span class="sb-quick-addr">${f.addr}</span>
              </span>
              <span class="sb-quick-star">★</span>
            </a>`).join('')}
        </div>

        <div class="sb-section">
          <div class="sb-section-header">
            <span class="sb-section-title">${t('recent')}</span>
          </div>
          ${DEFAULT_RECENT.map(r => `
            <a href="${r.href}" class="sb-quick-item" data-proto="${r.protocol}">
              <span class="sb-quick-icon" style="background:${PROTOCOL_COLORS[r.protocol]}15;color:${PROTOCOL_COLORS[r.protocol]}">${PROTOCOL_ICONS[r.protocol] || '·'}</span>
              <span class="sb-quick-info">
                <span class="sb-quick-name">${r.name}</span>
                <span class="sb-quick-addr">${r.addr}</span>
              </span>
              <span class="sb-quick-time">${r.time}</span>
            </a>`).join('')}
        </div>
      </nav>

      <div class="sidebar-footer">
        <a href="settings.html" class="nav-item ${activePage === 'settings' ? 'active' : ''}">
          <span class="nav-icon">⚙</span> <span>${t('settings')}</span>
        </a>
        <a href="environment-new.html" class="nav-item ${activePage === 'new-env' ? 'active' : ''}">
          <span class="nav-icon">+</span> <span>${t('newEnv')}</span>
        </a>
      </div>
      <button class="sidebar-collapse-btn" onclick="toggleSidebarCollapse()" title="折叠侧边栏">
        <span class="collapse-icon">◀</span>
        <span>折叠</span>
      </button>
    </aside>`;
}

// ─── Sidebar Interactions ───
function toggleEnvGroup(index) {
  const group = document.getElementById('env-resources-' + index);
  const chevron = document.getElementById('chevron-' + index);
  if (!group) return;
  const isOpen = group.classList.toggle('sb-env-open');
  if (chevron) chevron.textContent = isOpen ? '▾' : '▸';
}

function toggleSidebarCollapse() {
  const sidebar = document.getElementById('appSidebar');
  if (!sidebar) return;
  sidebar.classList.toggle('collapsed');
  localStorage.setItem('rex-sidebar-collapsed', sidebar.classList.contains('collapsed') ? '1' : '0');
  // Update main content margin
  const mainContent = document.querySelector('.main-content, .ws-main');
  if (mainContent) {
    mainContent.style.marginLeft = sidebar.classList.contains('collapsed') ? '48px' : '';
  }
}

function selectTheme(el, theme) {
  applyTheme(theme);
  updateThemeBtn();
  // Update settings page UI if present
  document.querySelectorAll('.theme-option').forEach(o => o.classList.remove('active'));
  if (el) el.classList.add('active');
}

function toggleTheme() {
  // Cycle: dark → light → auto → dark
  const cycle = { dark: 'light', light: 'auto', auto: 'dark' };
  applyTheme(cycle[currentTheme] || 'dark');
  updateThemeBtn();
}

function updateThemeBtn() {
  const btn = document.getElementById('themeBtn');
  if (!btn) return;
  const icons = { dark: '☀', light: '☾', auto: '◐' };
  btn.textContent = icons[currentTheme] || '☀';
  btn.title = { dark: 'Dark Mode', light: 'Light Mode', auto: 'Auto (System)' }[currentTheme] || '';
}

function toggleLang() {
  currentLang = currentLang === 'zh' ? 'en' : 'zh';
  localStorage.setItem('rex-lang', currentLang);
  const container = document.getElementById('sidebarContainer');
  if (container) {
    const activePage = container.getAttribute('data-active-page') || 'dashboard';
    container.innerHTML = buildSidebar(activePage);
    bindSidebarEvents();
  }
  document.querySelectorAll('[data-i18n]').forEach(el => {
    el.textContent = t(el.getAttribute('data-i18n'));
  });
}

function bindSidebarEvents() {
  // Mobile menu
  const menuBtn = document.querySelector('.mobile-menu-btn');
  const sidebar = document.getElementById('appSidebar');
  if (menuBtn && sidebar) {
    menuBtn.addEventListener('click', () => {
      sidebar.classList.toggle('sidebar-mobile-open');
    });
  }

  // Resource context menus
  document.querySelectorAll('.sb-resource-item').forEach(el => {
    el.addEventListener('contextmenu', (e) => {
      const name = el.dataset.resource;
      const proto = el.dataset.proto;
      showContextMenu(e, [
        { label: t('connect'), icon: '→', action: () => {} },
        { label: t('openSftpNewTab'), icon: '📁', action: () => {} },
        '---',
        { label: t('editResource'), icon: '✏', action: () => {} },
        { label: t('copyAddr'), icon: '📋', action: () => {} },
        '---',
        { label: t('deleteResource'), icon: '🗑', danger: true, action: () => {} },
      ]);
    });
  });

  // Environment header context menus
  document.querySelectorAll('.sb-env-header').forEach(el => {
    el.addEventListener('contextmenu', (e) => {
      const name = el.querySelector('.sb-env-name')?.textContent;
      showContextMenu(e, [
        { label: t('openDetail'), icon: '◎', action: () => {} },
        { label: t('openAllInWorkspace'), icon: '⊞', action: () => {} },
        '---',
        { label: t('newResource'), icon: '+', action: () => {} },
        { label: t('addAgent'), icon: '⬡', action: () => {} },
        '---',
        { label: t('editEnv'), icon: '✏', action: () => {} },
        { label: t('deleteEnv'), icon: '🗑', danger: true, action: () => {} },
      ]);
    });
  });

  // Favorites context menus
  document.querySelectorAll('.sb-quick-item').forEach(el => {
    el.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      const name = el.querySelector('.sb-quick-name')?.textContent;
      const isFav = !!el.querySelector('.sb-quick-star');
      showContextMenu(e, [
        { label: t('connect'), icon: '→', action: () => {} },
        '---',
        isFav
          ? { label: t('cancelFavorite'), icon: '☆', action: () => {} }
          : { label: '★ ' + t('favorites'), icon: '★', action: () => {} },
        { label: t('removeFromRecent'), icon: '✕', action: () => {} },
      ]);
    });
  });
}

// ─── Toast ───
function showToast(msg, type = 'success') {
  const toast = document.createElement('div');
  toast.className = 'toast toast-' + type;
  toast.innerHTML = '<span class="toast-icon">' + (type === 'success' ? '✓' : '✕') + '</span><span>' + msg + '</span>';
  document.body.appendChild(toast);
  setTimeout(() => { toast.style.opacity = '0'; toast.style.transition = 'opacity 0.3s'; }, 2500);
  setTimeout(() => toast.remove(), 3000);
}

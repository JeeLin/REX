/* REX prototype shared runtime — modal / confirm / toast / context menu / store
 * Loaded by every page. Pure prototype: data lives in localStorage per key. */
(function (global) {
  'use strict';

  /* ---------- helpers ---------- */
  function esc(s) { var d = document.createElement('div'); d.textContent = s; return d.innerHTML; }

  /* ---------- toast ---------- */
  function toast(msg, type) {
    type = type || 'ok';
    var host = document.querySelector('.rex-toasts');
    if (!host) { host = document.createElement('div'); host.className = 'rex-toasts'; document.body.appendChild(host); }
    var el = document.createElement('div');
    el.className = 'rex-toast ' + type;
    var icon = type === 'err' ? '✕' : type === 'info' ? 'ℹ' : '✓';
    var span = document.createElement('span'); span.textContent = msg;
    el.innerHTML = '<span class="ic">' + icon + '</span>';
    el.appendChild(span);
    host.appendChild(el);
    setTimeout(function () { el.style.opacity = '0'; el.style.transition = 'opacity .2s'; setTimeout(function () { el.remove(); }, 220); }, 2600);
  }

  /* ---------- modal (form) ---------- */
  function modal(opts) {
    // opts: { title, body(html), okText, onOk(values), onCancel, width }
    closeModal();
    var back = document.createElement('div');
    back.className = 'rex-modal-backdrop';
    back.innerHTML =
      '<div class="rex-modal" style="' + (opts.width ? 'width:' + opts.width : '') + '">' +
        '<div class="m-head"><h3>' + esc(opts.title || '') + '</h3><span class="rex-x" style="margin-left:auto;cursor:pointer;color:var(--text-muted)">✕</span></div>' +
        '<div class="m-body">' + opts.body + '</div>' +
        '<div class="m-foot">' +
          '<button class="rex-btn ghost rex-cancel">取消</button>' +
          '<button class="rex-btn primary rex-ok">' + (opts.okText || '保存') + '</button>' +
        '</div>' +
      '</div>';
    document.body.appendChild(back);
    requestAnimationFrame(function () { back.classList.add('show'); });
    function getValues() {
      var v = {};
      back.querySelectorAll('[data-field]').forEach(function (i) { v[i.dataset.field] = i.value; });
      return v;
    }
    back.querySelector('.rex-ok').addEventListener('click', function () {
      var ok = opts.onOk ? opts.onOk(getValues(), back) : true;
      if (ok !== false) closeModal();
    });
    back.querySelector('.rex-cancel').addEventListener('click', function () { if (opts.onCancel) opts.onCancel(); closeModal(); });
    back.querySelector('.rex-x').addEventListener('click', function () { if (opts.onCancel) opts.onCancel(); closeModal(); });
    back.addEventListener('click', function (e) { if (e.target === back) closeModal(); });
    return back;
  }
  function closeModal() { var b = document.querySelector('.rex-modal-backdrop'); if (b) b.remove(); }
  document.addEventListener('keydown', function (e) { if (e.key === 'Escape') closeModal(); });

  /* ---------- confirm ---------- */
  function confirm(opts, onYes) {
    // opts: { title, message, okText, danger }
    closeModal();
    var back = document.createElement('div');
    back.className = 'rex-modal-backdrop';
    back.innerHTML =
      '<div class="rex-modal rex-confirm">' +
        '<div class="m-head"><h3>' + esc(opts.title || '') + '</h3></div>' +
        '<div class="m-body"><p>' + esc(opts.message || '') + '</p></div>' +
        '<div class="m-body">' + opts.message + '</div>' +
        '<div class="m-foot">' +
          '<button class="rex-btn ghost rex-cancel">取消</button>' +
          '<button class="rex-btn ' + (opts.danger ? 'danger' : 'primary') + ' rex-yes">' + (opts.okText || '确定') + '</button>' +
        '</div>' +
      '</div>';
    document.body.appendChild(back);
    requestAnimationFrame(function () { back.classList.add('show'); });
    back.querySelector('.rex-yes').addEventListener('click', function () { closeModal(); onYes && onYes(); });
    back.querySelector('.rex-cancel').addEventListener('click', closeModal);
    back.addEventListener('click', function (e) { if (e.target === back) closeModal(); });
  }

  /* ---------- context menu ---------- */
  function contextMenu(items, x, y) {
    var m = document.querySelector('.rex-ctx');
    if (m) m.remove();
    m = document.createElement('div'); m.className = 'rex-ctx';
    items.forEach(function (it) {
      if (it.sep) { var s = document.createElement('div'); s.className = 'sep'; m.appendChild(s); return; }
      var b = document.createElement('button');
      b.textContent = it.label;
      if (it.danger) b.className = 'danger';
      b.addEventListener('click', function () { m.remove(); it.onClick && it.onClick(); });
      m.appendChild(b);
    });
    document.body.appendChild(m);
    m.classList.add('open');
    m.style.left = Math.min(x, window.innerWidth - m.offsetWidth - 8) + 'px';
    m.style.top = Math.min(y, window.innerHeight - m.offsetHeight - 8) + 'px';
    function kill() { m.remove(); document.removeEventListener('click', kill); }
    setTimeout(function () { document.addEventListener('click', kill); }, 0);
  }

  /* ---------- tiny local store ---------- */
  function store(key, initial) {
    return {
      get: function () {
        try { var v = JSON.parse(localStorage.getItem(key)); if (v != null) return v; } catch (e) {}
        return typeof initial === 'function' ? initial() : initial;
      },
      set: function (v) { try { localStorage.setItem(key, JSON.stringify(v)); } catch (e) {} },
      clear: function () { try { localStorage.removeItem(key); } catch (e) {} }
    };
  }

  /* ---------- shared "new resource" dialog ---------- */
  var RES_CREATE = {
    ssh:  '<div class="rex-field"><label>主机</label><input data-field="c_addr" placeholder="如 10.0.0.5"></div>' +
          '<div class="rex-field"><label>端口</label><input data-field="c_port" value="22"></div>' +
          '<div class="rex-field"><label>用户名</label><input data-field="c_user" placeholder="root"></div>' +
          '<div class="rex-field"><label>认证方式</label><select data-field="c_auth"><option>密码</option><option>密钥</option></select></div>',
    sql:  '<div class="rex-field"><label>主机</label><input data-field="c_addr" placeholder="如 10.0.4.12"></div>' +
          '<div class="rex-field"><label>端口</label><input data-field="c_port" value="3306"></div>' +
          '<div class="rex-field"><label>数据库</label><input data-field="c_db" placeholder="shop"></div>' +
          '<div class="rex-field"><label>用户名</label><input data-field="c_user" placeholder="root"></div>',
    redis:'<div class="rex-field"><label>主机</label><input data-field="c_addr" placeholder="如 10.0.4.13"></div>' +
          '<div class="rex-field"><label>端口</label><input data-field="c_port" value="6379"></div>' +
          '<div class="rex-field"><label>DB 索引</label><input data-field="c_db" value="0"></div>' +
          '<div class="rex-field"><label>密码（可选）</label><input data-field="c_pass" type="password"></div>',
    files:'<div class="rex-field"><label>主机</label><input data-field="c_addr" placeholder="如 192.168.1.20"></div>' +
          '<div class="rex-field"><label>端口</label><input data-field="c_port" value="22"></div>' +
          '<div class="rex-field"><label>用户名</label><input data-field="c_user" placeholder="root"></div>' +
          '<div class="rex-field"><label>初始路径</label><input data-field="c_path" value="/"></div>',
    s3:   '<div class="rex-field"><label>Endpoint</label><input data-field="c_endpoint" placeholder="https://s3.xxx.com"></div>' +
          '<div class="rex-field"><label>Bucket</label><input data-field="c_bucket" placeholder="assets"></div>' +
          '<div class="rex-field"><label>Region</label><input data-field="c_region" placeholder="cn-east-1"></div>' +
          '<div class="rex-field"><label>Access Key</label><input data-field="c_ak" placeholder="AKIA…"></div>',
    sip:  '<div class="rex-field"><label>服务器地址</label><input data-field="c_addr" placeholder="如 pbx.local"></div>' +
          '<div class="rex-field"><label>端口</label><input data-field="c_port" value="5060"></div>' +
          '<div class="rex-field"><label>分机 / 账号</label><input data-field="c_ext" placeholder="1001"></div>' +
          '<div class="rex-field"><label>密码</label><input data-field="c_pass" type="password"></div>'
  };
  function newResource(envName) {
    var m = modal({
      title: '新建资源', width: '520px', okText: '创建',
      body:
        '<div class="rex-field"><label>资源名称</label><input data-field="name" placeholder="如 db-replica"></div>' +
        '<div class="rex-field"><label>类型</label><select class="res-kind-select" data-field="kind"><option value="ssh">SSH</option><option value="sql">SQL</option><option value="redis">Redis</option><option value="files">SFTP</option><option value="s3">Object Storage</option><option value="sip">SIP</option></select></div>' +
        '<div class="res-fields"></div>' +
        '<div class="rex-field"><label>所属环境</label><select data-field="env"><option>Aliyun Prod</option><option>Home LAN</option><option>Lab VPN</option><option>Object Storage</option></select></div>' +
        '<div class="rex-test"><button type="button" class="rex-btn ghost rex-test-btn">测试连接</button><span class="rex-test-msg muted" style="margin-left:10px;font-size:12.5px"></span></div>',
      onOk: function (v) {
        if (!v.name) { toast('请填写资源名称', 'err'); return false; }
        var addr = v.c_addr || v.c_endpoint;
        if (!addr) { toast('请填写主机 / 地址', 'err'); return false; }
        var params = {};
        ['c_addr','c_port','c_user','c_auth','c_db','c_path','c_endpoint','c_bucket','c_region','c_ak','c_ext','c_pass']
          .forEach(function (k) { if (v[k] != null && String(v[k]).trim() !== '') params[k] = v[k]; });
        var list = REX.store('rex.connections.v1', function () { return []; }).get();
        list.push({ kind: v.kind, name: v.name, env: v.env, params: params });
        REX.store('rex.connections.v1', function () { return []; }).set(list);
        toast('资源「' + v.name + '」已创建（所属：' + (v.env || '—') + '）' + (REX.openConnection ? ' 并打开' : ''), 'ok');
        if (REX.openConnection) REX.openConnection(v.kind, v.name);
        return true;
      }
    });
    var fieldsEl = m.querySelector('.res-fields');
    function fill(kind) { fieldsEl.innerHTML = RES_CREATE[kind] || ''; }
    fill('ssh');
    m.querySelector('.res-kind-select').addEventListener('change', function () { fill(this.value); });
    // 内联「测试连接」：在同一弹窗内校验关键字段后 mock 返回结果（不关闭弹窗）
    var testBtn = m.querySelector('.rex-test-btn');
    var testMsg = m.querySelector('.rex-test-msg');
    if (testBtn) testBtn.addEventListener('click', function () {
      var kind = m.querySelector('.res-kind-select').value;
      var addr = (m.querySelector('[data-field="c_addr"]') || m.querySelector('[data-field="c_endpoint"]') || {}).value;
      if (!addr) { testMsg.textContent = '请先填写主机 / 地址'; testMsg.style.color = 'var(--danger)'; return; }
      testBtn.disabled = true; testBtn.textContent = '连接中…';
      testMsg.textContent = '正在连接 ' + addr + ' …'; testMsg.style.color = 'var(--text-muted)';
      setTimeout(function () {
        testBtn.disabled = false; testBtn.textContent = '测试连接';
        testMsg.textContent = '✓ 连接成功（' + kind.toUpperCase() + ' · 演示）'; testMsg.style.color = 'var(--success)';
      }, 700);
    });
    // 预选所属环境（侧栏 / 卡片「+」传入当前环境）
    if (envName) {
      var sel = m.querySelector('[data-field="env"]');
      if (sel) {
        var match = Array.prototype.find.call(sel.options, function (o) { return o.value === envName; });
        if (match) sel.value = envName;
        else { var opt = document.createElement('option'); opt.value = envName; opt.textContent = envName; sel.appendChild(opt); sel.value = envName; }
      }
    }
    return m;
  }

  /* ---------- shared "new environment" dialog ---------- */
  function newEnvironment() {
    modal({
      title: '新建环境', width: '480px', okText: '创建',
      body: '<div class="rex-field"><label>环境名称</label><input data-field="name" placeholder="如 Aliyun Prod"></div>' +
            '<div class="rex-field"><label>描述</label><textarea data-field="desc" rows="2" placeholder="可选"></textarea></div>' +
            '<div class="rex-field"><label>连接方式</label><select data-field="mode"><option>直连</option><option>经 Agent 隧道</option></select></div>',
      onOk: function (v) { if (!v.name) { toast('请填写名称', 'err'); return false; } toast('环境「' + v.name + '」已创建', 'ok'); return true; }
    });
  }

  /* ---------- global chrome actions (data-rex-action) ----------
   * Any element with data-rex-action="theme|lang|new-env" is handled here so
   * individual pages don't each re-wire the same global buttons. */
  document.addEventListener('click', function (e) {
    var el = e.target.closest('[data-rex-action]');
    if (!el) return;
    var a = el.getAttribute('data-rex-action');
    if (a === 'theme') toast('主题切换（演示）', 'info');
    else if (a === 'lang') toast('语言切换（演示）', 'info');
    else if (a === 'new-env') newEnvironment();
    else if (a === 'new-res') newResource();
  });

  /* ---------- shared sidebar wiring ----------
   * The environment list (.side-envs) and the resource-tree group headers
   * (.grp) both expose a per-environment 「+」 that opens the new-resource
   * dialog scoped to that environment. Centralized so every page shares it. */
  function initSidebar() {
    document.querySelectorAll('.side-env').forEach(function (el) {
      var name = el.getAttribute('data-env');
      if (!name) return;
      var add = el.querySelector('.env-add');
      if (add) add.addEventListener('click', function (e) { e.stopPropagation(); REX.newResource(name); });
      el.addEventListener('click', function () { location.href = '07b-environment-detail.html?env=' + encodeURIComponent(name); });
    });
    document.querySelectorAll('.grp[data-env] .add, .grp[data-env] .env-add').forEach(function (btn) {
      btn.addEventListener('click', function (e) {
        e.stopPropagation();
        var g = btn.closest('.grp');
        if (g) REX.newResource(g.getAttribute('data-env'));
      });
    });
  }

  /* ---------- mobile nav drawer ----------
   * On ≤760px the sidebar is a slide-in drawer; inject a hamburger into the
   * visible top bar (or the workspace group bar) and toggle it with a scrim. */
  function initMobileNav() {
    var app = document.querySelector('.app');
    if (!app) return;
    var sidebar = app.querySelector('.sidebar');
    if (!sidebar) return;
    var scrim = document.querySelector('.rex-scrim');
    if (!scrim) { scrim = document.createElement('div'); scrim.className = 'rex-scrim'; document.body.appendChild(scrim); }
    function open() { sidebar.classList.add('open'); scrim.classList.add('show'); }
    function close() { sidebar.classList.remove('open'); scrim.classList.remove('show'); }
    scrim.addEventListener('click', close);
    var bar = document.querySelector('.topbar');
    if (!bar) bar = document.getElementById('groupbar');
    if (bar) {
      var btn = document.createElement('button');
      btn.className = 'menu-btn';
      btn.setAttribute('aria-label', 'Menu');
      btn.innerHTML = '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 6h18M3 12h18M3 18h18"/></svg>';
      btn.addEventListener('click', function (e) { e.stopPropagation(); open(); });
      bar.insertBefore(btn, bar.firstChild);
    }
    sidebar.querySelectorAll('.nav a, .side-foot .btn').forEach(function (a) { a.addEventListener('click', close); });
    document.addEventListener('keydown', function (e) { if (e.key === 'Escape') close(); });
  }
  initMobileNav();
  initSidebar();

  /* ---------- command palette (Cmd/Ctrl+K) ---------- */
  function go(url) { location.href = url; }
  var PALETTE_COMMANDS = [
    { cat: '导航', icon: '▦', label: '前往 工作区', desc: 'Workspace', run: function () { go('02-workspace.html'); } },
    { cat: '导航', icon: '▤', label: '前往 仪表盘', desc: 'Dashboard', run: function () { go('01-dashboard.html'); } },
    { cat: '导航', icon: '▤', label: '前往 环境', desc: 'Environments', run: function () { go('07-environments.html'); } },
    { cat: '导航', icon: '⟡', label: '前往 代理', desc: 'Agents', run: function () { go('08-agents.html'); } },
    { cat: '导航', icon: '☰', label: '前往 审计日志', desc: 'Audit log', run: function () { go('10-audit.html'); } },
    { cat: '导航', icon: '⚙', label: '前往 设置', desc: 'Settings', run: function () { go('09-settings.html'); } },
    { cat: '导航', icon: '◳', label: '设计系统预览', desc: 'Design preview', run: function () { go('12-design-preview.html'); } },
    { cat: '操作', icon: '＋', label: '新建资源', desc: 'New resource', run: function () { REX.newResource(); } },
    { cat: '操作', icon: '＋', label: '新建环境', desc: 'New environment', run: function () { REX.newEnvironment(); } },
    { cat: '操作', icon: '⟡', label: '注册 Agent', desc: 'Register agent', run: function () { REX.modal({ title: '注册 Agent', width: '440px', okText: '生成命令', body: '<div class="rex-field"><label>Agent 名称</label><input data-field="an" placeholder="如 edge-gw"></div><div class="rex-field"><label>绑定环境</label><select data-field="env"><option>Aliyun Prod</option><option>Home LAN</option><option>Lab VPN</option></select></div><div class="rex-field" data-cmd-wrap style="display:none"><label>在目标主机执行以下命令</label><div style="display:flex;gap:8px;align-items:center"><input data-cmd readonly class="mono" style="flex:1;height:36px;padding:0 11px;border-radius:8px;border:1px solid var(--border-strong);background:var(--bg-elevated);color:var(--text);font:inherit;font-size:12.5px"><button type="button" class="rex-btn ghost rex-copy" style="height:36px">复制</button></div><p class="hint muted" style="font-size:11px;color:var(--text-dim);margin:6px 0 0">命令仅当前浏览器会话可见，不含真实密钥。</p><p class="hint muted" style="font-size:11px;color:var(--text-dim);margin:6px 0 0">命令格式为原型示意（register / --name / --env / --token 为占位），最终子命令与参数名以 rex-agent CLI 文档为准。</p></div>', onOk: function (v) {
        if (!v.an) { toast('请填写名称', 'err'); return false; }
        var an = v.an.trim(), env = v.env || '';
        var token = 'ra_' + Math.random().toString(36).slice(2, 10) + Math.random().toString(36).slice(2, 10);
        var cmd = 'rex-agent register --name ' + an + (env ? ' --env ' + env : '') + ' --token ' + token;
        // render the command back into the modal body (do not close)
        var back = document.querySelector('.rex-modal-backdrop');
        if (back) {
          var cmdWrap = back.querySelector('[data-cmd-wrap]');
          var cmdInput = back.querySelector('[data-cmd]');
          if (cmdWrap && cmdInput) {
            cmdInput.value = cmd;
            cmdWrap.style.display = '';
            var copyBtn = back.querySelector('.rex-copy');
            if (copyBtn) copyBtn.onclick = function () {
              try { (navigator.clipboard || { writeText: function (t) { var ta = document.createElement('textarea'); ta.value = t; document.body.appendChild(ta); ta.select(); document.execCommand('copy'); ta.remove(); } }).writeText(cmd); } catch (e) {}
              toast('已复制到剪贴板', 'ok');
            };
          }
        }
        toast('命令已生成（在弹窗内可复制）', 'ok');
        return false; // keep modal open so user can copy
      } }); } },
    { cat: '操作', icon: '⟳', label: '刷新数据', desc: 'Refresh', run: function () { toast('已刷新（演示数据）', 'ok'); } },
    { cat: '演示', icon: '⚠', label: '模拟 Token 过期', desc: 'Expire token', run: function () { try { localStorage.setItem('rex-token', 'expired'); } catch (e) {} REX.tokenRefresh(); } }
  ];
  function commandPalette() {
    closeModal();
    var back = document.createElement('div'); back.className = 'rex-palette-backdrop';
    back.innerHTML =
      '<div class="rex-palette">' +
        '<div class="rex-palette-input"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="7"/><path d="m21 21-4-4"/></svg><input placeholder="搜索命令…  (↑↓ 选择 · Enter 执行 · Esc 关闭)"></div>' +
        '<div class="rex-palette-list"></div>' +
        '<div class="rex-palette-foot"><span><span class="kbd">↑↓</span> 选择</span><span><span class="kbd">Enter</span> 执行</span><span><span class="kbd">Esc</span> 关闭</span></div>' +
      '</div>';
    document.body.appendChild(back);
    requestAnimationFrame(function () { back.classList.add('show'); });
    var input = back.querySelector('input');
    var listEl = back.querySelector('.rex-palette-list');
    var flat = [], cur = 0;
    function render(q) {
      q = (q || '').trim().toLowerCase();
      flat = PALETTE_COMMANDS.filter(function (c) { return !q || c.label.toLowerCase().indexOf(q) >= 0 || c.desc.toLowerCase().indexOf(q) >= 0; });
      if (!flat.length) { listEl.innerHTML = '<div class="rex-palette-empty">无匹配命令</div>'; return; }
      var html = '', last = '';
      flat.forEach(function (c, i) {
        if (c.cat !== last) { html += '<div class="rex-palette-cat">' + c.cat + '</div>'; last = c.cat; }
        html += '<div class="rex-palette-item' + (i === 0 ? ' sel' : '') + '" data-i="' + i + '"><span class="pi">' + c.icon + '</span><span class="pl">' + c.label + '</span><span class="pd">' + c.desc + '</span></div>';
      });
      listEl.innerHTML = html;
    }
    function setCur(i) {
      cur = i;
      listEl.querySelectorAll('.rex-palette-item').forEach(function (el) { el.classList.toggle('sel', +el.dataset.i === cur); });
      var el = listEl.querySelector('.rex-palette-item.sel'); if (el) el.scrollIntoView({ block: 'nearest' });
    }
    function run(i) { var c = flat[i]; if (c) { back.remove(); c.run(); } }
    render('');
    setTimeout(function () { input.focus(); }, 30);
    input.addEventListener('input', function () { render(input.value); cur = 0; setCur(0); });
    listEl.addEventListener('click', function (e) { var it = e.target.closest('.rex-palette-item'); if (it) run(+it.dataset.i); });
    input.addEventListener('keydown', function (e) {
      if (e.key === 'ArrowDown') { e.preventDefault(); if (flat.length) setCur(Math.min(flat.length - 1, cur + 1)); }
      else if (e.key === 'ArrowUp') { e.preventDefault(); if (flat.length) setCur(Math.max(0, cur - 1)); }
      else if (e.key === 'Enter') { e.preventDefault(); run(cur); }
    });
    back.addEventListener('click', function (e) { if (e.target === back) back.remove(); });
  }

  /* ---------- session timeout / auto-lock ---------- */
  var SESSION_TIMEOUT_MS = 10 * 60 * 1000; // demo: 10 min (product default 120)
  var SESSION_WARN_MS = 60 * 1000;
  var _sessTimer = null, _lastAct = Date.now(), _warned = false;
  function _act() { _lastAct = Date.now(); _warned = false; }
  function _tickSess() {
    var idle = Date.now() - _lastAct;
    if (idle >= SESSION_TIMEOUT_MS) { stopSession(); try { localStorage.removeItem('rex-token'); } catch (e) {} location.href = '00-login.html?reason=timeout'; }
    else if (idle >= SESSION_TIMEOUT_MS - SESSION_WARN_MS && !_warned) { _warned = true; toast('即将因空闲自动锁定', 'info'); }
  }
  function startSession(ms) {
    SESSION_TIMEOUT_MS = ms || SESSION_TIMEOUT_MS; _lastAct = Date.now();
    document.addEventListener('mousemove', _act); document.addEventListener('keydown', _act);
    if (_sessTimer) clearInterval(_sessTimer);
    _sessTimer = setInterval(_tickSess, 5000);
  }
  function stopSession() { if (_sessTimer) clearInterval(_sessTimer); _sessTimer = null; document.removeEventListener('mousemove', _act); document.removeEventListener('keydown', _act); }

  /* ---------- token refresh (in-place, no redirect to login) ---------- */
  function tokenRefresh() {
    closeModal();
    var back = document.createElement('div'); back.className = 'rex-modal-backdrop';
    back.innerHTML =
      '<div class="rex-modal" style="width:420px">' +
        '<div class="m-head"><h3>会话已过期</h3></div>' +
        '<div class="m-body">' +
          '<p class="muted" style="font-size:13px;margin:0 0 14px">登录令牌已失效。输入密码即可在当前页面恢复会话，无需跳回登录页。</p>' +
          '<div class="rex-field"><label>密码</label><input data-field="pw" type="password" placeholder="输入当前账户密码" autofocus></div>' +
        '</div>' +
        '<div class="m-foot"><button class="rex-btn ghost rex-cancel">重新登录</button><button class="rex-btn primary rex-refresh">恢复会话</button></div>' +
      '</div>';
    document.body.appendChild(back);
    requestAnimationFrame(function () { back.classList.add('show'); });
    var pwInput = back.querySelector('[data-field="pw"]');
    if (pwInput) setTimeout(function () { pwInput.focus(); }, 60);
    function doRefresh() {
      if (!pwInput.value) { toast('请输入密码', 'err'); pwInput.focus(); return; }
      if (pwInput.value.length < 6) { toast('密码至少 6 位', 'err'); pwInput.focus(); return; }
      try { localStorage.setItem('rex-token', 'valid'); } catch (e) {}
      closeModal();
      if (typeof startSession === 'function') startSession();
      toast('会话已恢复', 'ok');
    }
    back.querySelector('.rex-refresh').addEventListener('click', doRefresh);
    if (pwInput) pwInput.addEventListener('keydown', function (e) { if (e.key === 'Enter') { e.preventDefault(); doRefresh(); } });
    back.querySelector('.rex-cancel').addEventListener('click', function () { try { localStorage.removeItem('rex-token'); } catch (e) {} closeModal(); location.href = '00-login.html'; });
  }
  function checkToken() { try { if (localStorage.getItem('rex-token') === 'expired') tokenRefresh(); } catch (e) {} }

  // global shortcut: Cmd/Ctrl+K → command palette
  document.addEventListener('keydown', function (e) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); REX.commandPalette(); }
  });
  // decorative search boxes open the palette
  document.querySelectorAll('.search .box, .search-inline').forEach(function (b) { b.style.cursor = 'pointer'; b.addEventListener('click', function () { REX.commandPalette(); }); });

  // session + token (skip on login / setup gate pages)
  var _path = location.pathname.split('/').pop();
  if (_path !== '00-login.html' && _path !== '03-setup.html') {
    try { if (localStorage.getItem('rex-token') !== 'expired') startSession(); } catch (e) {}
    checkToken();
  }

  /* ---------- SQL tools (§2.1) ---------- */
  function sqlTableDesigner() {
    var ddl = "CREATE TABLE orders (\n  id BIGINT PRIMARY KEY AUTO_INCREMENT,\n  status VARCHAR(16) NOT NULL,\n  total DECIMAL(10,2) NOT NULL,\n  created_at DATETIME\n);";
    modal({
      title: '表设计器', width: '560px', okText: '保存',
      body:
        '<div class="rex-field"><label>表名</label><input data-field="tname" value="orders"></div>' +
        '<div class="rex-field"><label>DDL 预览</label><textarea data-field="ddl" rows="8" style="font-family:var(--font-mono);font-size:12px">' + ddl + '</textarea></div>',
      onOk: function () { toast('表结构已更新（演示）', 'ok'); return true; }
    });
  }
  function sqlImport() {
    modal({ title: '导入数据', width: '460px', okText: '导入',
      body: '<div class="rex-field"><label>格式</label><select data-field="fmt"><option>CSV</option><option>JSON</option><option>SQL</option></select></div>' +
            '<div class="rex-field"><label>源文件 / 文本</label><textarea data-field="src" rows="4" placeholder="粘贴或选择文件…"></textarea></div>',
      onOk: function () { toast('导入任务已创建（演示）', 'ok'); return true; } });
  }
  function sqlExport() {
    modal({ title: '导出数据', width: '460px', okText: '导出',
      body: '<div class="rex-field"><label>格式</label><select data-field="fmt"><option>CSV</option><option>JSON</option><option>SQL</option></select></div>' +
            '<div class="rex-field"><label>行范围</label><input data-field="range" placeholder="如 1..1000"></div>',
      onOk: function () { toast('导出任务已创建（演示）', 'ok'); return true; } });
  }
  var SAVED_Q = 'rex.saved-queries.v1';
  function savedQueries() {
    var st = REX.store(SAVED_Q, function () { return []; }), list = st.get();
    var items = list.length
      ? list.map(function (q, i) { return '<div style="display:flex;gap:8px;align-items:center;margin-bottom:8px"><span style="flex:1;font-family:var(--font-mono);font-size:12.5px">' + q + '</span><button class="rex-btn ghost" data-del="' + i + '" style="height:28px">删</button></div>'; }).join('')
      : '<p class="muted" style="font-size:13px">暂无已存查询</p>';
    modal({ title: '已存查询', width: '460px', okText: '关闭',
      body: '<div>' + items + '</div>' +
            '<div class="rex-field" style="margin-top:12px"><label>保存当前编辑器 SQL（命名）</label><input data-field="newq" placeholder="如 查近 7 天订单"></div>',
      onOk: function () { return true; } });
    var back = document.querySelector('.rex-modal-backdrop');
    if (back) {
      back.querySelectorAll('[data-del]').forEach(function (b) {
        b.addEventListener('click', function () {
          var i = +b.dataset.del, l = st.get(); l.splice(i, 1); st.set(l);
          REX.toast('已删除', 'ok'); savedQueries();
        });
      });
    }
  }
  function globalQuery() {
    modal({ title: '全局查询', width: '480px', okText: '运行',
      body: '<div class="rex-field"><label>跨库 SQL</label><textarea data-field="gq" rows="4" style="font-family:var(--font-mono);font-size:12px">SELECT * FROM information_schema.tables LIMIT 50;</textarea></div>',
      onOk: function () { toast('已跨库执行（演示）', 'ok'); return true; } });
  }

  /* ---------- Files / object storage (§2.3) ---------- */
  function fileEditor(name, content) {
    content = content || '-- 文件内容（演示）\nSELECT 1;';
    modal({ title: '编辑文件 · ' + (name || ''), width: '600px', okText: '保存',
      body: '<textarea data-field="fc" rows="12" style="width:100%;font-family:var(--font-mono);font-size:12.5px;background:var(--bg-terminal);color:#c9d4df;border:1px solid var(--border-strong);border-radius:8px;padding:10px">' + content + '</textarea>',
      onOk: function () { toast('文件已保存（演示）', 'ok'); return true; } });
  }
  function folderSync() {
    modal({ title: '文件夹同步', width: '480px', okText: '开始同步',
      body: '<div class="rex-field"><label>源</label><input data-field="src" placeholder="本地 /srv/www"></div>' +
            '<div class="rex-field"><label>目标</label><input data-field="dst" placeholder="sftp://nas.local/srv/www"></div>' +
            '<div class="rex-field"><label>方向</label><select data-field="dir"><option>上传 ↑</option><option>下载 ↓</option></select></div>',
      onOk: function () { toast('同步任务已创建（数据不经过浏览器）', 'ok'); return true; } });
  }

  /* ---------- SIP CDR drawer (§2.4, folded into SIP panel) ---------- */
  function cdrDrawer() {
    var rows = [
      { t: '08-23 14:22', a: '1001', b: '13800001234', d: 'out', s: 'answered', dur: '02:14' },
      { t: '08-23 13:10', a: '13800005678', b: '1002', d: 'in', s: 'missed', dur: '00:00' },
      { t: '08-22 22:08', a: '1003', b: '13900008888', d: 'out', s: 'answered', dur: '05:41' }
    ];
    var body = '<div class="rex-field"><label>筛选</label><div style="display:flex;gap:8px">' +
      '<select data-field="fd" style="flex:1"><option value="">全部方向</option><option value="out">呼出</option><option value="in">呼入</option></select>' +
      '<select data-field="fs" style="flex:1"><option value="">全部状态</option><option value="answered">已接</option><option value="missed">未接</option></select></div></div>' +
      '<table class="t" style="width:100%;border-collapse:collapse;font-size:12.5px">' +
      '<thead><tr><th style="text-align:left;padding:8px 10px;color:var(--text-dim);font-size:11px">时间</th><th style="text-align:left;padding:8px 10px;color:var(--text-dim);font-size:11px">主叫</th><th style="text-align:left;padding:8px 10px;color:var(--text-dim);font-size:11px">被叫</th><th style="text-align:left;padding:8px 10px;color:var(--text-dim);font-size:11px">方向</th><th style="text-align:left;padding:8px 10px;color:var(--text-dim);font-size:11px">状态</th><th style="text-align:left;padding:8px 10px;color:var(--text-dim);font-size:11px">时长</th></tr></thead><tbody>' +
      rows.map(function (r) {
        return '<tr><td style="padding:8px 10px;font-family:var(--font-mono)">' + r.t + '</td><td style="padding:8px 10px;font-family:var(--font-mono)">' + r.a + '</td><td style="padding:8px 10px;font-family:var(--font-mono)">' + r.b + '</td><td style="padding:8px 10px">' + (r.d === 'out' ? '呼出' : '呼入') + '</td><td style="padding:8px 10px">' + (r.s === 'answered' ? '已接' : '未接') + '</td><td style="padding:8px 10px;font-family:var(--font-mono)">' + r.dur + '</td></tr>';
      }).join('') + '</tbody></table>';
    modal({ title: '通话记录 (CDR)', width: '640px', okText: '关闭', body: body,
      onOk: function () { return true; } });
  }

  /* ---------- Settings: check for update (§3) ---------- */
  function checkForUpdate() {
    toast('正在检查更新…', 'info');
    setTimeout(function () { toast('已是最新：rex-hub 1.0.0 / rex-agent 1.0.0', 'ok'); }, 600);
  }

  /* ---------- Favorites / recent (§3) ---------- */
  var FAV_KEY = 'rex.favorites.v1';
  function toggleFavorite(id) {
    var st = REX.store(FAV_KEY, function () { return []; }), l = st.get(), i = l.indexOf(id);
    if (i >= 0) { l.splice(i, 1); toast('已取消收藏', 'info'); } else { l.push(id); toast('已收藏', 'ok'); }
    st.set(l);
  }

  global.REX = {
    toast: toast, modal: modal, confirm: confirm, contextMenu: contextMenu, store: store,
    closeModal: closeModal, newEnvironment: newEnvironment, newResource: newResource,
    commandPalette: commandPalette, tokenRefresh: tokenRefresh, startSession: startSession, stopSession: stopSession,
    sqlTableDesigner: sqlTableDesigner, sqlImport: sqlImport, sqlExport: sqlExport, savedQueries: savedQueries, globalQuery: globalQuery,
    fileEditor: fileEditor, folderSync: folderSync, cdrDrawer: cdrDrawer, checkForUpdate: checkForUpdate, toggleFavorite: toggleFavorite,
    RES_CREATE: RES_CREATE
  };
})(window);

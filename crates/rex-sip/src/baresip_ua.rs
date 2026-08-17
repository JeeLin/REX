//! `SipUa` 安全封装：包真 baresip FFI 实现（`BaresipSipUa`）与测试 Mock 分发。
//!
//! M82a（v0.70.0）只打通信令层：register/dial/answer/hangup/hold/unhold/dtmf +
//! 事件流（由 baresip `bevent` 事件总线映射）。baresip 主循环（`re_main`）跑在**单一**
//! OS 线程，所有 `ua_*`/`call_*` 控制 API 也必须在那条线程上调用（baresip/libre 单线程
//! 主循环模型）。控制操作经 baresip 的 `mqueue` 序列化到 `re_main` 线程执行，避免跨线程
//! 数据竞争；事件则经 `bevent` 回调映射为 [`SipEvent`] 广播回各订阅者。
//! 音频回调（M82b）在事件线程内接管原始 RTP 帧。

use crate::{mock::MockSipUa, CallState, SipConfig, SipEvent, SipTransport, SipUaTrait};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::sync::oneshot;

// baresip / re 自动生成绑定中的类型与函数（来自 `crate::bindings` 的 include）。
use crate::{
    account_set_auth_pass, account_set_auth_user, audio_bridge, audio_set_player, audio_set_source,
    baresip_init, bevent_ev_BEVENT_CALL_CLOSED, bevent_ev_BEVENT_CALL_ESTABLISHED,
    bevent_ev_BEVENT_CALL_HOLD, bevent_ev_BEVENT_CALL_INCOMING, bevent_ev_BEVENT_CALL_RESUME,
    bevent_ev_BEVENT_CALL_RINGING, bevent_ev_BEVENT_REGISTER_FAIL, bevent_ev_BEVENT_REGISTER_OK,
    bevent_get_call, bevent_get_text, bevent_register, call, call_audio, call_hold,
    call_send_digit, mqueue, mqueue_alloc, mqueue_push, re_main, ua, ua_account, ua_alloc,
    ua_answer, ua_connect, ua_hangup, ua_register, ua_stop_register, vidmode_VIDMODE_OFF,
};
use crate::audio_bridge::AudioBridge;

/// 全局 bevent 回调的共享状态：事件 sink + call_id→call* 映射 + 主线程 mqueue。
struct BaresipState {
    txs: Mutex<Vec<mpsc::UnboundedSender<SipEvent>>>,
    calls: Mutex<HashMap<String, *mut call>>,
    /// 唯一 `re_main` 线程服务的 mqueue，所有控制操作经它序列化到主线程执行。
    mq: *mut mqueue,
    /// 音频桥接（M82b）：自定义 baresip 音频驱动 state + RX/TX PCM 队列。
    audio: Arc<AudioBridge>,
}

unsafe impl Send for BaresipState {}
unsafe impl Sync for BaresipState {}

static STATE: Mutex<Option<Arc<BaresipState>>> = Mutex::new(None);

/// 安全的 `SipUa` 句柄：构造时选择真 baresip 实现或 Mock。
pub enum SipUa {
    Real(BaresipSipUa),
    Mock(MockSipUa),
}

impl SipUa {
    /// 构造真 baresip UA（进程内 FFI）。
    pub async fn real(cfg: SipConfig) -> Result<Self> {
        Ok(SipUa::Real(BaresipSipUa::new(cfg).await?))
    }

    /// 构造测试 Mock UA。
    pub fn mock(cfg: SipConfig, injected: Vec<SipEvent>) -> Self {
        SipUa::Mock(MockSipUa::new(cfg, injected))
    }
}

#[async_trait]
impl SipUaTrait for SipUa {
    async fn register(&self) -> Result<()> {
        match self {
            SipUa::Real(u) => u.register().await,
            SipUa::Mock(u) => u.register().await,
        }
    }
    async fn dial(&self, dest: &str) -> Result<String> {
        match self {
            SipUa::Real(u) => u.dial(dest).await,
            SipUa::Mock(u) => u.dial(dest).await,
        }
    }
    async fn answer(&self, call_id: &str) -> Result<()> {
        match self {
            SipUa::Real(u) => u.answer(call_id).await,
            SipUa::Mock(u) => u.answer(call_id).await,
        }
    }
    async fn hangup(&self, call_id: &str) -> Result<()> {
        match self {
            SipUa::Real(u) => u.hangup(call_id).await,
            SipUa::Mock(u) => u.hangup(call_id).await,
        }
    }
    async fn hold(&self, call_id: &str) -> Result<()> {
        match self {
            SipUa::Real(u) => u.hold(call_id).await,
            SipUa::Mock(u) => u.hold(call_id).await,
        }
    }
    async fn unhold(&self, call_id: &str) -> Result<()> {
        match self {
            SipUa::Real(u) => u.unhold(call_id).await,
            SipUa::Mock(u) => u.unhold(call_id).await,
        }
    }
    async fn dtmf(&self, call_id: &str, digit: char) -> Result<()> {
        match self {
            SipUa::Real(u) => u.dtmf(call_id, digit).await,
            SipUa::Mock(u) => u.dtmf(call_id, digit).await,
        }
    }
    fn events(&self) -> mpsc::UnboundedReceiver<SipEvent> {
        match self {
            SipUa::Real(u) => u.events(),
            SipUa::Mock(u) => u.events(),
        }
    }
    fn on_rtp(&self, cb: Box<dyn FnMut(&[i16]) + Send + 'static>) {
        match self {
            SipUa::Real(u) => u.on_rtp(cb),
            SipUa::Mock(u) => u.on_rtp(cb),
        }
    }
    async fn send_audio(&self, pcm: Vec<i16>) -> anyhow::Result<()> {
        match self {
            SipUa::Real(u) => u.send_audio(pcm).await,
            SipUa::Mock(u) => u.send_audio(pcm).await,
        }
    }
}

/// 真 baresip FFI 封装。
pub struct BaresipSipUa {
    state: Arc<BaresipState>,
    ua: Mutex<*mut ua>,
}

// 原始 UA 指针仅在持有 Mutex 下访问，且所有 baresip 控制 API 都经 mqueue 在主线程执行，
// 故可安全标记 Send/Sync。
unsafe impl Send for BaresipSipUa {}
unsafe impl Sync for BaresipSipUa {}

impl BaresipSipUa {
    pub async fn new(cfg: SipConfig) -> Result<Self> {
        // 确保唯一的 baresip 运行时（re_main + mqueue）只初始化一次；首次之外的调用
        // 直接复用全局运行时，避免重复 baresip_init / 重复 re_main 线程争抢全局实例。
        let mq = ensure_runtime()?;
        // 账户建立（ua_alloc + 认证信息）也属 baresip 内部状态操作，必须序列化到主线程。
        let (tx, rx) = oneshot::channel();
        let msg = Box::new(CtrlOp::Setup {
            username: cfg.username,
            server: cfg.server,
            port: cfg.port,
            password: cfg.password,
            transport: cfg.transport,
            result_tx: tx,
        });
        unsafe {
            mqueue_push(mq, 0, Box::into_raw(msg) as *mut c_void);
        }
        let ua = recv_ctrl(rx, "setup").await? as *mut ua;
        let state = STATE
            .lock()
            .unwrap()
            .as_ref()
            .expect("runtime initialized")
            .clone();
        Ok(Self {
            state,
            ua: Mutex::new(ua),
        })
    }
}

/// 保证 baresip 运行时单例：首次调用执行 `baresip_init`、注册 bevent/mqueue、spawn `re_main`；
/// 后续调用直接返回已有的 mqueue 指针。
fn ensure_runtime() -> Result<*mut mqueue> {
    let mut g = STATE.lock().unwrap();
    if let Some(s) = g.as_ref() {
        return Ok(s.mq);
    }
    unsafe {
        bevent_register(Some(bevent_cb), std::ptr::null_mut());
        let rc = baresip_init(std::ptr::null_mut());
        if rc != 0 {
            return Err(anyhow!("baresip_init failed: {rc}"));
        }
        let mut mq: *mut mqueue = std::ptr::null_mut();
        let rc = mqueue_alloc(&mut mq, Some(mq_handler), std::ptr::null_mut());
        if rc != 0 || mq.is_null() {
            return Err(anyhow!("mqueue_alloc failed: {rc}"));
        }
        let s = Arc::new(BaresipState {
            txs: Mutex::new(Vec::new()),
            calls: Mutex::new(HashMap::new()),
            mq,
            audio: Arc::new(AudioBridge::new()),
        });
        // 注册自定义 baresip 音频驱动（ausrc/auplay），接管 RTP↔PCM 搬运（M82b）。
        // 注册失败即回收 STATE 并返回错误，避免后续 UA 在缺音频驱动下建立通话。
        if let Err(e) = audio_bridge::register_audio_drivers(&s.audio) {
            *g = None;
            return Err(e);
        }
        *g = Some(s.clone());
        // 启动唯一的 re_main 事件循环线程（baresip 单 OS 线程主循环）。
        std::thread::spawn(|| {
            re_main(None);
        });
        Ok(mq)
    }
}

/// 接收主线程经 mqueue 回传的控制操作结果。
///
/// 通道内层错误为 `String`（baresip 侧的错误描述），统一转为 `anyhow::Error` 作为
/// 函数返回；若 oneshot 被丢弃（主线程已退出），报 "sip {op} channel dropped"。
async fn recv_ctrl<T>(rx: oneshot::Receiver<Result<T, String>>, op: &str) -> Result<T> {
    match rx.await {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(anyhow!("sip {op} failed: {e}")),
        Err(_) => Err(anyhow!("sip {op} channel dropped")),
    }
}

#[async_trait]
impl SipUaTrait for BaresipSipUa {
    async fn register(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        {
            // 裸指针仅在 mqueue_push 前需要；push 后即离开作用域，
            // 避免被 await 捕获导致 future 因裸指针 !Send 而无法跨线程。
            let ua = *self.ua.lock().unwrap();
            let msg = Box::new(CtrlOp::Register { ua, result_tx: tx });
            unsafe {
                mqueue_push(self.state.mq, 0, Box::into_raw(msg) as *mut c_void);
            }
        }
        recv_ctrl(rx, "register").await
    }

    async fn dial(&self, dest: &str) -> Result<String> {
        let (tx, rx) = oneshot::channel();
        {
            let ua = *self.ua.lock().unwrap();
            let msg = Box::new(CtrlOp::Dial {
                ua,
                dest: dest.to_string(),
                result_tx: tx,
            });
            unsafe {
                mqueue_push(self.state.mq, 0, Box::into_raw(msg) as *mut c_void);
            }
        }
        recv_ctrl(rx, "dial").await
    }

    async fn answer(&self, call_id: &str) -> Result<()> {
        let call = self.lookup_call(call_id)?;
        let (tx, rx) = oneshot::channel();
        {
            let ua = *self.ua.lock().unwrap();
            let msg = Box::new(CtrlOp::Answer {
                ua,
                call,
                result_tx: tx,
            });
            unsafe {
                mqueue_push(self.state.mq, 0, Box::into_raw(msg) as *mut c_void);
            }
        }
        recv_ctrl(rx, "answer").await
    }

    async fn hangup(&self, call_id: &str) -> Result<()> {
        let call = self.lookup_call(call_id)?;
        let (tx, rx) = oneshot::channel();
        {
            let ua = *self.ua.lock().unwrap();
            let msg = Box::new(CtrlOp::Hangup {
                ua,
                call,
                result_tx: tx,
            });
            unsafe {
                mqueue_push(self.state.mq, 0, Box::into_raw(msg) as *mut c_void);
            }
        }
        let r = recv_ctrl(rx, "hangup").await;
        // 控制操作已在主线程完成，此处仅做 Rust 侧 map 清理（Mutex 保护，跨线程安全）。
        self.state.calls.lock().unwrap().remove(call_id);
        r
    }

    async fn hold(&self, call_id: &str) -> Result<()> {
        self.ctrl_call(call_id, true).await
    }

    async fn unhold(&self, call_id: &str) -> Result<()> {
        self.ctrl_call(call_id, false).await
    }

    async fn dtmf(&self, call_id: &str, digit: char) -> Result<()> {
        let call = self.lookup_call(call_id)?;
        let (tx, rx) = oneshot::channel();
        {
            let msg = Box::new(CtrlOp::Dtmf {
                call,
                digit: digit as c_char,
                result_tx: tx,
            });
            unsafe {
                mqueue_push(self.state.mq, 0, Box::into_raw(msg) as *mut c_void);
            }
        }
        recv_ctrl(rx, "dtmf").await
    }

    fn events(&self) -> mpsc::UnboundedReceiver<SipEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.state.txs.lock().unwrap().push(tx);
        rx
    }

    /// 注册远端→浏览器 PCM 回调：直接挂到共享 `AudioBridge`（M82b）。泵线程每帧 RX
    /// PCM 到达即触发；回调在 baresip 泵线程内同步调用。
    fn on_rtp(&self, cb: Box<dyn FnMut(&[i16]) + Send + 'static>) {
        self.state.audio.set_on_rtp(Box::new(cb));
    }

    /// 浏览器麦克风 PCM 回传：压入 `AudioBridge` 的 TX 队列，由泵线程取走去编码发对端。
    async fn send_audio(&self, pcm: Vec<i16>) -> Result<()> {
        self.state.audio.push_tx(pcm);
        Ok(())
    }
}

impl BaresipSipUa {
    async fn ctrl_call(&self, call_id: &str, hold: bool) -> Result<()> {
        let call = self.lookup_call(call_id)?;
        let (tx, rx) = oneshot::channel();
        {
            let msg = Box::new(CtrlOp::Hold {
                call,
                hold,
                result_tx: tx,
            });
            unsafe {
                mqueue_push(self.state.mq, 0, Box::into_raw(msg) as *mut c_void);
            }
        }
        recv_ctrl(rx, "hold").await
    }

    fn lookup_call(&self, call_id: &str) -> Result<*mut call> {
        self.state
            .calls
            .lock()
            .unwrap()
            .get(call_id)
            .copied()
            .ok_or_else(|| anyhow!("unknown call_id: {call_id}"))
    }
}

impl Drop for BaresipSipUa {
    fn drop(&mut self) {
        let ua = *self.ua.lock().unwrap();
        if !ua.is_null() {
            unsafe { ua_stop_register(ua) };
        }
    }
}

/// 经 mqueue 序列化到 `re_main` 线程执行的控制操作。
///
/// 每个变体携带一个 `oneshot` 回传结果给发起的异步任务。跨线程所有权经
/// `Box::into_raw`/`Box::from_raw` 在 mqueue 边界传递，handler 运行在主线程。
enum CtrlOp {
    Setup {
        username: String,
        server: String,
        port: u16,
        password: Option<String>,
        transport: SipTransport,
        // 回传 ua 指针的整数形式(usize)以保持 oneshot 通道 Send；
        // 裸指针 *mut ua 不 Send，会拖累 new() 的 future 跨线程。
        result_tx: oneshot::Sender<Result<usize, String>>,
    },
    Register {
        ua: *mut ua,
        result_tx: oneshot::Sender<Result<(), String>>,
    },
    Dial {
        ua: *mut ua,
        dest: String,
        result_tx: oneshot::Sender<Result<String, String>>,
    },
    Answer {
        ua: *mut ua,
        call: *mut call,
        result_tx: oneshot::Sender<Result<(), String>>,
    },
    Hangup {
        ua: *mut ua,
        call: *mut call,
        result_tx: oneshot::Sender<Result<(), String>>,
    },
    Hold {
        call: *mut call,
        hold: bool,
        result_tx: oneshot::Sender<Result<(), String>>,
    },
    Dtmf {
        call: *mut call,
        digit: c_char,
        result_tx: oneshot::Sender<Result<(), String>>,
    },
}

/// `re_main` 线程上执行的控制分发：每个 `ua_*`/`call_*` 调用都在主线程上进行，消除跨线程竞争。
unsafe extern "C" fn mq_handler(_id: c_int, data: *mut c_void, _arg: *mut c_void) {
    if data.is_null() {
        return;
    }
    let msg = Box::from_raw(data as *mut CtrlOp);
    match *msg {
        CtrlOp::Setup {
            username,
            server,
            port,
            password,
            transport,
            result_tx,
        } => {
            let r = setup_account(&username, &server, port, password.as_deref(), transport)
                .map(|p| p as usize);
            let _ = result_tx.send(r);
        }
        CtrlOp::Register { ua, result_tx } => {
            let r = if ua.is_null() {
                Err("baresip UA not initialized".into())
            } else {
                let rc = ua_register(ua);
                if rc == 0 {
                    Ok(())
                } else {
                    Err(format!("ua_register failed: {rc}"))
                }
            };
            let _ = result_tx.send(r);
        }
        CtrlOp::Dial {
            ua,
            dest,
            result_tx,
        } => {
            let _ = result_tx.send(dial_call(ua, &dest));
        }
        CtrlOp::Answer {
            ua,
            call,
            result_tx,
        } => {
            let r = if ua.is_null() || call.is_null() {
                Err("invalid ua/call".into())
            } else {
                let rc = ua_answer(ua, call, vidmode_VIDMODE_OFF);
                if rc == 0 {
                    Ok(())
                } else {
                    Err(format!("ua_answer failed: {rc}"))
                }
            };
            let _ = result_tx.send(r);
        }
        CtrlOp::Hangup {
            ua,
            call,
            result_tx,
        } => {
            if !ua.is_null() && !call.is_null() {
                ua_hangup(ua, call, 0, std::ptr::null());
            }
            let _ = result_tx.send(Ok(()));
        }
        CtrlOp::Hold {
            call,
            hold,
            result_tx,
        } => {
            let r = if call.is_null() {
                Err("invalid call".into())
            } else {
                let rc = call_hold(call, hold);
                if rc == 0 {
                    Ok(())
                } else {
                    Err(format!("call_hold failed: {rc}"))
                }
            };
            let _ = result_tx.send(r);
        }
        CtrlOp::Dtmf {
            call,
            digit,
            result_tx,
        } => {
            let r = if call.is_null() {
                Err("invalid call".into())
            } else {
                let rc = call_send_digit(call, digit);
                if rc == 0 {
                    Ok(())
                } else {
                    Err(format!("call_send_digit failed: {rc}"))
                }
            };
            let _ = result_tx.send(r);
        }
    }
}

/// 在主线程上建立 baresip 账户（ua_alloc + 认证）。AOR 携带 port 与 transport。
unsafe fn setup_account(
    username: &str,
    server: &str,
    port: u16,
    password: Option<&str>,
    transport: SipTransport,
) -> Result<*mut ua, String> {
    let mut aor = format!("sip:{}@{}", username, server);
    if port != 5060 {
        aor.push_str(&format!(":{port}"));
    }
    match transport {
        SipTransport::Udp => {}
        SipTransport::Tcp => aor.push_str(";transport=tcp"),
        SipTransport::Tls => aor.push_str(";transport=tls"),
    }
    let aor_c = CString::new(aor).map_err(|e| format!("invalid aor: {e}"))?;
    let mut ua_ptr: *mut ua = std::ptr::null_mut();
    let rc = ua_alloc(&mut ua_ptr, aor_c.as_ptr());
    if rc != 0 || ua_ptr.is_null() {
        return Err(format!("ua_alloc failed: {rc}"));
    }
    let acc = ua_account(ua_ptr);
    if !acc.is_null() {
        if let Some(pass) = password {
            if let Ok(p) = CString::new(pass) {
                account_set_auth_pass(acc, p.as_ptr());
            }
        }
        if let Ok(u) = CString::new(username) {
            account_set_auth_user(acc, u.as_ptr());
        }
    }
    Ok(ua_ptr)
}

/// 在主线程上发起拨号，并把新 call 指针登记进 `calls` map。
unsafe fn dial_call(ua: *mut ua, dest: &str) -> Result<String, String> {
    if ua.is_null() {
        return Err("baresip UA not initialized".into());
    }
    let dest_c = CString::new(dest).map_err(|e| format!("invalid destination: {e}"))?;
    let mut call: *mut call = std::ptr::null_mut();
    let rc = ua_connect(
        ua,
        &mut call,
        std::ptr::null(),
        dest_c.as_ptr(),
        vidmode_VIDMODE_OFF,
    );
    if rc != 0 || call.is_null() {
        return Err(format!("ua_connect failed: {rc}"));
    }
    let id = format!("call-{:p}", call);
    if let Some(s) = STATE.lock().unwrap().as_ref() {
        s.calls.lock().unwrap().insert(id.clone(), call);
    }
    Ok(id)
}

/// 全局 bevent 回调：映射 baresip 事件到 [`SipEvent`] 并广播给订阅者。
///
/// **不持有 `STATE` 锁跨 `map_bevent`**——先取出 `Arc` 副本释放锁，再映射；`map_bevent`
/// 内部如需改 `calls` map 会自行加锁，避免递归死锁。
unsafe extern "C" fn bevent_cb(ev: crate::bevent_ev, event: *mut crate::bevent, _arg: *mut c_void) {
    let state = {
        let g = STATE.lock().unwrap();
        match g.as_ref() {
            Some(s) => s.clone(),
            None => return,
        }
    };
    if let Some(sip_ev) = map_bevent(ev, event) {
        let mut txs = state.txs.lock().unwrap();
        txs.retain(|tx| tx.send(sip_ev.clone()).is_ok());
    }
}

/// baresip `bevent_ev` → [`SipEvent`] 映射。运行在 `re_main` 线程。
unsafe fn map_bevent(ev: crate::bevent_ev, event: *mut crate::bevent) -> Option<SipEvent> {
    match ev {
        bevent_ev_BEVENT_REGISTER_OK => Some(SipEvent::Registered),
        bevent_ev_BEVENT_REGISTER_FAIL => {
            let reason = event_text(event);
            Some(SipEvent::RegistrationFailed { reason })
        }
        bevent_ev_BEVENT_CALL_INCOMING => {
            let call = bevent_get_call(event);
            let id = format!("call-{:p}", call);
            if let Some(s) = STATE.lock().unwrap().as_ref() {
                s.calls.lock().unwrap().insert(id.clone(), call);
            }
            let from = event_text(event);
            Some(SipEvent::IncomingCall { call_id: id, from })
        }
        bevent_ev_BEVENT_CALL_RINGING => Some(call_state(event, CallState::Ringing)),
        bevent_ev_BEVENT_CALL_ESTABLISHED => {
            // 通话建立：把自定义 ausrc/auplay 驱动挂到该 call 的 audio 上，接管 RTP↔PCM
            // （M82b）。本回调运行在 baresip `re_main` 线程，可安全调用 audio_* API。
            let call = bevent_get_call(event);
            if !call.is_null() {
                unsafe {
                    let au = call_audio(call);
                    if !au.is_null() {
                        let src = CString::new(crate::audio_bridge::DRIVER_NAME_AUSRC)
                            .unwrap_or_default();
                        let play = CString::new(crate::audio_bridge::DRIVER_NAME_AUPLAY)
                            .unwrap_or_default();
                        let dev = CString::new(crate::audio_bridge::DEVICE_NAME).unwrap_or_default();
                        audio_set_source(au, src.as_ptr(), dev.as_ptr());
                        audio_set_player(au, play.as_ptr(), dev.as_ptr());
                    }
                }
            }
            Some(call_state(event, CallState::Active))
        }
        bevent_ev_BEVENT_CALL_CLOSED => {
            let call = bevent_get_call(event);
            let id = format!("call-{:p}", call);
            // 通话关闭即释放，立即从 map 移除，避免后续 hold/dtmf 解引用悬垂指针（UAF）。
            if let Some(s) = STATE.lock().unwrap().as_ref() {
                s.calls.lock().unwrap().remove(&id);
            }
            Some(SipEvent::CallState {
                call_id: id,
                state: CallState::Ended,
            })
        }
        bevent_ev_BEVENT_CALL_HOLD => Some(call_state(event, CallState::Held)),
        bevent_ev_BEVENT_CALL_RESUME => Some(call_state(event, CallState::Active)),
        _ => None,
    }
}

unsafe fn call_state(event: *mut crate::bevent, state: CallState) -> SipEvent {
    let call = bevent_get_call(event);
    let id = format!("call-{:p}", call);
    SipEvent::CallState { call_id: id, state }
}

unsafe fn event_text(event: *mut crate::bevent) -> String {
    let p = bevent_get_text(event);
    if p.is_null() {
        String::new()
    } else {
        CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_ua_via_enum_constructs() {
        let ua = SipUa::mock(
            SipConfig {
                server: "s".into(),
                port: 5060,
                username: "u".into(),
                password: Some("p".into()),
                display_name: None,
                transport: crate::SipTransport::Udp,
            },
            vec![],
        );
        let _rx = ua.events();
    }
}

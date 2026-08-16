//! `SipUa` 安全封装：包真 baresip FFI 实现（`BaresipSipUa`）与测试 Mock 分发。
//!
//! M82a（v0.70.0）只打通信令层：register/dial/answer/hangup/hold/unhold/dtmf +
//! 事件流（由 baresip `bevent` 事件总线映射）。baresip 主循环（`re_main`）跑在独立
//! OS 线程，事件经 `bevent` 回调映射到 [`SipEvent`] 推入 channel 回主异步运行时。
//! 音频回调（M82b）在事件线程内接管原始 RTP 帧。

use crate::{mock::MockSipUa, CallState, SipConfig, SipEvent, SipUaTrait};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

// baresip / re 自动生成绑定中的类型与函数（来自 `crate::bindings` 的 include）。
use crate::{
    account_set_auth_pass, account_set_auth_user, baresip_init, bevent_ev_BEVENT_CALL_CLOSED,
    bevent_ev_BEVENT_CALL_ESTABLISHED, bevent_ev_BEVENT_CALL_HOLD, bevent_ev_BEVENT_CALL_INCOMING,
    bevent_ev_BEVENT_CALL_RESUME, bevent_ev_BEVENT_CALL_RINGING, bevent_ev_BEVENT_REGISTER_FAIL,
    bevent_ev_BEVENT_REGISTER_OK, bevent_get_call, bevent_get_text, bevent_register, call,
    call_hold, call_send_digit, ua, ua_account, ua_alloc, ua_answer, ua_connect, ua_hangup,
    ua_register, ua_stop_register, vidmode_VIDMODE_OFF,
};

/// 全局 bevent 回调的共享状态：事件 sink + call_id→call* 映射。
struct BaresipState {
    txs: Mutex<Vec<mpsc::UnboundedSender<SipEvent>>>,
    calls: Mutex<HashMap<String, *mut call>>,
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
    pub fn real(cfg: SipConfig) -> Result<Self> {
        Ok(SipUa::Real(BaresipSipUa::new(cfg)?))
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
    fn on_rtp(&self, cb: impl FnMut(&[u8]) + Send + 'static) {
        match self {
            SipUa::Real(u) => u.on_rtp(cb),
            SipUa::Mock(u) => u.on_rtp(cb),
        }
    }
}

/// 真 baresip FFI 封装。
pub struct BaresipSipUa {
    state: Arc<BaresipState>,
    ua: Mutex<*mut ua>,
}

// 原始 UA 指针仅在持有 Mutex 下访问，或在本 crate 拉起的 baresip 线程内访问；
// baresip 本身是单 OS 线程模型，跨线程只传递事件 channel，故可安全标记 Send/Sync。
unsafe impl Send for BaresipSipUa {}
unsafe impl Sync for BaresipSipUa {}

impl BaresipSipUa {
    pub fn new(cfg: SipConfig) -> Result<Self> {
        let state = {
            let mut g = STATE.lock().unwrap();
            match g.as_ref() {
                Some(s) => s.clone(),
                None => {
                    let s = Arc::new(BaresipState {
                        txs: Mutex::new(Vec::new()),
                        calls: Mutex::new(HashMap::new()),
                    });
                    // 安装全局 bevent 回调（baresip 单实例事件总线）。
                    unsafe {
                        bevent_register(Some(bevent_cb), std::ptr::null_mut());
                    }
                    *g = Some(s.clone());
                    s
                }
            }
        };

        // 初始化 baresip（空 config 用默认）。
        let rc = unsafe { baresip_init(std::ptr::null_mut()) };
        if rc != 0 {
            return Err(anyhow!("baresip_init failed: {rc}"));
        }

        let mut ua_ptr: *mut ua = std::ptr::null_mut();
        let aor = format!("sip:{}@{}", cfg.username, cfg.server);
        let aor_c = CString::new(aor)?;
        let rc = unsafe { ua_alloc(&mut ua_ptr, aor_c.as_ptr()) };
        if rc != 0 || ua_ptr.is_null() {
            return Err(anyhow!("ua_alloc failed: {rc}"));
        }
        // 填认证信息。
        unsafe {
            let acc = ua_account(ua_ptr);
            if !acc.is_null() {
                if let Some(pass) = &cfg.password {
                    if let Ok(pass) = CString::new(pass.clone()) {
                        account_set_auth_pass(acc, pass.as_ptr());
                    }
                }
                if let Ok(user) = CString::new(cfg.username.clone()) {
                    account_set_auth_user(acc, user.as_ptr());
                }
            }
        }

        // 启动 re_main 事件循环线程（baresip 单 OS 线程主循环）。
        std::thread::spawn(|| unsafe {
            crate::re_main(None);
        });

        Ok(Self {
            state,
            ua: Mutex::new(ua_ptr),
        })
    }
}

#[async_trait]
impl SipUaTrait for BaresipSipUa {
    async fn register(&self) -> Result<()> {
        let ua = *self.ua.lock().unwrap();
        if ua.is_null() {
            return Err(anyhow!("baresip UA not initialized"));
        }
        let rc = unsafe { ua_register(ua) };
        if rc != 0 {
            return Err(anyhow!("ua_register failed: {rc}"));
        }
        Ok(())
    }

    async fn dial(&self, dest: &str) -> Result<String> {
        let ua = *self.ua.lock().unwrap();
        if ua.is_null() {
            return Err(anyhow!("baresip UA not initialized"));
        }
        let dest_c = CString::new(dest)?;
        let mut call: *mut call = std::ptr::null_mut();
        let rc = unsafe {
            ua_connect(
                ua,
                &mut call,
                std::ptr::null(),
                dest_c.as_ptr(),
                vidmode_VIDMODE_OFF,
            )
        };
        if rc != 0 || call.is_null() {
            return Err(anyhow!("ua_connect failed: {rc}"));
        }
        let id = format!("call-{:p}", call);
        self.state.calls.lock().unwrap().insert(id.clone(), call);
        Ok(id)
    }

    async fn answer(&self, call_id: &str) -> Result<()> {
        let call = self.lookup_call(call_id)?;
        let ua = *self.ua.lock().unwrap();
        let rc = unsafe { ua_answer(ua, call, vidmode_VIDMODE_OFF) };
        if rc != 0 {
            return Err(anyhow!("ua_answer failed: {rc}"));
        }
        Ok(())
    }

    async fn hangup(&self, call_id: &str) -> Result<()> {
        let call = self.lookup_call(call_id)?;
        let ua = *self.ua.lock().unwrap();
        unsafe { ua_hangup(ua, call, 0, std::ptr::null()) };
        self.state.calls.lock().unwrap().remove(call_id);
        Ok(())
    }

    async fn hold(&self, call_id: &str) -> Result<()> {
        let call = self.lookup_call(call_id)?;
        let rc = unsafe { call_hold(call, true) };
        if rc != 0 {
            return Err(anyhow!("call_hold failed: {rc}"));
        }
        Ok(())
    }

    async fn unhold(&self, call_id: &str) -> Result<()> {
        let call = self.lookup_call(call_id)?;
        let rc = unsafe { call_hold(call, false) };
        if rc != 0 {
            return Err(anyhow!("call_hold(resume) failed: {rc}"));
        }
        Ok(())
    }

    async fn dtmf(&self, call_id: &str, digit: char) -> Result<()> {
        let call = self.lookup_call(call_id)?;
        let rc = unsafe { call_send_digit(call, digit as c_char) };
        if rc != 0 {
            return Err(anyhow!("call_send_digit failed: {rc}"));
        }
        Ok(())
    }

    fn events(&self) -> mpsc::UnboundedReceiver<SipEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.state.txs.lock().unwrap().push(tx);
        rx
    }
}

impl BaresipSipUa {
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

/// 全局 bevent 回调：映射 baresip 事件到 [`SipEvent`] 并广播给订阅者。
unsafe extern "C" fn bevent_cb(ev: crate::bevent_ev, event: *mut crate::bevent, _arg: *mut c_void) {
    let state = STATE.lock().unwrap();
    let Some(state) = state.as_ref() else {
        return;
    };
    if let Some(sip_ev) = map_bevent(ev, event) {
        let mut txs = state.txs.lock().unwrap();
        txs.retain(|tx| tx.send(sip_ev.clone()).is_ok());
    }
}

/// baresip `bevent_ev` → [`SipEvent`] 映射。
unsafe fn map_bevent(ev: crate::bevent_ev, event: *mut crate::bevent) -> Option<SipEvent> {
    match ev {
        bevent_ev_BEVENT_REGISTER_OK => Some(SipEvent::Registered),
        bevent_ev_BEVENT_REGISTER_FAIL => {
            let reason = event_text(event);
            Some(SipEvent::RegistrationFailed(reason))
        }
        bevent_ev_BEVENT_CALL_INCOMING => {
            let call = bevent_get_call(event);
            let id = format!("call-{:p}", call);
            state_calls_insert(&id, call);
            let from = event_text(event);
            Some(SipEvent::IncomingCall { call_id: id, from })
        }
        bevent_ev_BEVENT_CALL_RINGING => Some(call_state(event, CallState::Ringing)),
        bevent_ev_BEVENT_CALL_ESTABLISHED => Some(call_state(event, CallState::Active)),
        bevent_ev_BEVENT_CALL_CLOSED => Some(call_state(event, CallState::Ended)),
        bevent_ev_BEVENT_CALL_HOLD => Some(call_state(event, CallState::Held)),
        bevent_ev_BEVENT_CALL_RESUME => Some(call_state(event, CallState::Active)),
        _ => None,
    }
}

unsafe fn state_calls_insert(id: &str, call: *mut call) {
    if let Some(s) = STATE.lock().unwrap().as_ref() {
        s.calls.lock().unwrap().insert(id.to_string(), call);
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

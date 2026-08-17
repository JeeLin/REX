//! 测试用 `SipUaTrait` 实现：不依赖真 baresip 栈，确定性驱动事件流，供
//! Hub / Agent 的 handler 单测与无真 SIP server 场景使用。

use crate::{SipConfig, SipEvent, SipUaTrait};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};
use tokio::sync::mpsc;

/// `MockSipUa`：用 channel 回放预设事件；`dial` 等动作返回确定性 `call_id`。
pub struct MockSipUa {
    next_call: AtomicUsize,
    /// 注入的事件队列（FIFO），`events()` 创建 receiver 时一次性倒给订阅者。
    injected: Mutex<Vec<SipEvent>>,
    /// 记录收到的控制动作，便于断言。
    pub actions: Mutex<Vec<MockAction>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MockAction {
    Register,
    Dial(String),
    Answer(String),
    Hangup(String),
    Hold(String),
    Unhold(String),
    Dtmf(String, char),
    SendAudio(usize),
}

impl MockSipUa {
    /// 用 config + 预设事件构造；`events()` 订阅者会收到这些事件（每个 receiver 一份拷贝）。
    pub fn new(cfg: SipConfig, injected: Vec<SipEvent>) -> Self {
        let _ = cfg; // config 在 M82a Mock 中仅用于构造语义，动作不依赖它
        Self {
            next_call: AtomicUsize::new(1),
            injected: Mutex::new(injected),
            actions: Mutex::new(Vec::new()),
        }
    }

    /// 便捷构造：注册成功后立即产生一个来电。
    pub fn with_incoming_call(cfg: SipConfig, from: &str) -> Self {
        let call_id = "call-1".to_string();
        Self::new(
            cfg,
            vec![
                SipEvent::Registered,
                SipEvent::IncomingCall {
                    call_id,
                    from: from.to_string(),
                },
            ],
        )
    }

    fn record(&self, a: MockAction) {
        self.actions.lock().unwrap().push(a);
    }
}

#[async_trait]
impl SipUaTrait for MockSipUa {
    async fn register(&self) -> Result<()> {
        self.record(MockAction::Register);
        Ok(())
    }

    async fn dial(&self, dest: &str) -> Result<String> {
        let id = format!("call-{}", self.next_call.fetch_add(1, Ordering::SeqCst));
        self.record(MockAction::Dial(dest.to_string()));
        Ok(id)
    }

    async fn answer(&self, call_id: &str) -> Result<()> {
        self.record(MockAction::Answer(call_id.to_string()));
        Ok(())
    }

    async fn hangup(&self, call_id: &str) -> Result<()> {
        self.record(MockAction::Hangup(call_id.to_string()));
        Ok(())
    }

    async fn hold(&self, call_id: &str) -> Result<()> {
        self.record(MockAction::Hold(call_id.to_string()));
        Ok(())
    }

    async fn unhold(&self, call_id: &str) -> Result<()> {
        self.record(MockAction::Unhold(call_id.to_string()));
        Ok(())
    }

    async fn dtmf(&self, call_id: &str, digit: char) -> Result<()> {
        self.record(MockAction::Dtmf(call_id.to_string(), digit));
        Ok(())
    }

    fn on_rtp(&self, _cb: Box<dyn FnMut(&[i16]) + Send + 'static>) {
        // Mock 不依赖真 baresip，仅接收回调，不触发（handler 单测直接驱动 RX）。
    }

    async fn send_audio(&self, pcm: Vec<i16>) -> Result<()> {
        self.record(MockAction::SendAudio(pcm.len()));
        Ok(())
    }

    fn events(&self) -> mpsc::UnboundedReceiver<SipEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        for ev in self.injected.lock().unwrap().iter() {
            let _ = tx.send(ev.clone());
        }
        rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> SipConfig {
        SipConfig {
            server: "sip.example.com".into(),
            port: 5060,
            username: "1000".into(),
            password: Some("secret".into()),
            display_name: None,
            transport: crate::SipTransport::Udp,
        }
    }

    #[tokio::test]
    async fn register_dial_emit_actions() {
        let ua = MockSipUa::new(cfg(), vec![]);
        ua.register().await.unwrap();
        let id = ua.dial("2000").await.unwrap();
        assert_eq!(id, "call-1");
        let acts = ua.actions.lock().unwrap();
        assert_eq!(acts[0], MockAction::Register);
        assert_eq!(acts[1], MockAction::Dial("2000".into()));
    }

    #[tokio::test]
    async fn events_replay_injected() {
        let ua = MockSipUa::with_incoming_call(cfg(), "2000@example.com");
        let mut rx = ua.events();
        assert_eq!(rx.recv().await, Some(SipEvent::Registered));
        match rx.recv().await {
            Some(SipEvent::IncomingCall { call_id, from }) => {
                assert_eq!(call_id, "call-1");
                assert_eq!(from, "2000@example.com");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[tokio::test]
    async fn hold_unhold_dtmf_actions() {
        let ua = MockSipUa::new(cfg(), vec![]);
        ua.hold("call-1").await.unwrap();
        ua.unhold("call-1").await.unwrap();
        ua.dtmf("call-1", '5').await.unwrap();
        let acts = ua.actions.lock().unwrap();
        assert_eq!(acts[0], MockAction::Hold("call-1".into()));
        assert_eq!(acts[1], MockAction::Unhold("call-1".into()));
        assert_eq!(acts[2], MockAction::Dtmf("call-1".into(), '5'));
    }
}

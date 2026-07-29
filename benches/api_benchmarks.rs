use criterion::{criterion_group, criterion_main, Criterion};

fn bench_json_serialize(c: &mut Criterion) {
    c.bench_function("json_serialize_small", |b| {
        let data = serde_json::json!({
            "type": "heartbeat",
            "payload": {
                "version": "0.54.0",
                "os": "linux",
                "arch": "amd64",
                "hostname": "test-server"
            }
        });
        b.iter(|| serde_json::to_string(&data).unwrap());
    });
}

fn bench_json_deserialize(c: &mut Criterion) {
    c.bench_function("json_deserialize_heartbeat", |b| {
        let json = r#"{"type":"heartbeat_ack"}"#;
        b.iter(|| serde_json::from_str::<serde_json::Value>(json).unwrap());
    });
}

criterion_group!(benches, bench_json_serialize, bench_json_deserialize);
criterion_main!(benches);

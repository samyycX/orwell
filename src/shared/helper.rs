use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_now_timestamp() -> u64 {
    let now = SystemTime::now();
    now.duration_since(UNIX_EPOCH).unwrap().as_secs()
}

#[macro_export]
macro_rules! decode_packet {
    ($packet:expr, $packet_type:ty) => {{
        let decoded = <$packet_type>::decode($packet.data.as_slice());
        if decoded.is_err() {
            return Err(anyhow::anyhow!("非法数据"));
        }
        decoded.unwrap()
    }};
}

pub fn color_code_to_hex(color: i32) -> String {
    format!("#{:06X}", color)
}

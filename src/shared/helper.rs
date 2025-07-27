use chrono::{DateTime, FixedOffset, Utc};
use sha2::Sha256;
use sha3::Digest;

const VERSION: u64 = 1;

pub fn get_now_timestamp() -> u64 {
    let utc_plus_8 = FixedOffset::east_opt(8 * 3600).unwrap();
    let now_utc8: DateTime<FixedOffset> = Utc::now().with_timezone(&utc_plus_8);
    now_utc8.timestamp_millis() as u64
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
    format!("#{color:06X}")
}

pub fn get_version() -> u64 {
    VERSION
}

pub fn get_hash_version() -> String {
    let mut hasher = Sha256::new();
    hasher.update(get_version().to_le_bytes());
    format!("{:X}", hasher.finalize())
}

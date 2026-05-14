use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyType {
    Byte,
    Int16,
    Int32,
    Float,
    String,
    Array,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertyValue {
    Byte(u8),
    Int16(i16),
    Int32(i32),
    Float(f32),
    String(String),
    Array(Vec<u8>),
}

impl PropertyValue {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            PropertyValue::Byte(v) => vec![*v],
            PropertyValue::Int16(v) => v.to_be_bytes().to_vec(),
            PropertyValue::Int32(v) => v.to_be_bytes().to_vec(),
            PropertyValue::Float(v) => {
                let fixed = to_8_24(*v);
                fixed.to_be_bytes().to_vec()
            }
            PropertyValue::String(s) => {
                let mut buf = [0u8; 32];
                let bytes = s.as_bytes();
                let len = bytes.len().min(31);
                buf[..len].copy_from_slice(&bytes[..len]);
                buf.to_vec()
            }
            PropertyValue::Array(v) => v.clone(),
        }
    }

    pub fn decode(prop_type: PropertyType, data: &[u8]) -> Option<Self> {
        match prop_type {
            PropertyType::Byte => data.first().map(|&v| PropertyValue::Byte(v)),
            PropertyType::Int16 => {
                if data.len() >= 2 {
                    Some(PropertyValue::Int16(i16::from_be_bytes([data[0], data[1]])))
                } else {
                    None
                }
            }
            PropertyType::Int32 => {
                if data.len() >= 4 {
                    Some(PropertyValue::Int32(i32::from_be_bytes([
                        data[0], data[1], data[2], data[3],
                    ])))
                } else {
                    None
                }
            }
            PropertyType::Float => {
                if data.len() >= 4 {
                    let fixed = i32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                    Some(PropertyValue::Float(from_8_24(fixed)))
                } else {
                    None
                }
            }
            PropertyType::String => {
                let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
                let s = std::str::from_utf8(&data[..end]).ok()?;
                Some(PropertyValue::String(s.to_string()))
            }
            PropertyType::Array => Some(PropertyValue::Array(data.to_vec())),
        }
    }
}

const FIXED_POINT_SCALE: f32 = 0x0100_0000 as f32;

fn to_8_24(v: f32) -> i32 {
    (v * FIXED_POINT_SCALE).round() as i32
}

fn from_8_24(v: i32) -> f32 {
    v as f32 / FIXED_POINT_SCALE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_point_roundtrip() {
        let values = [0.0f32, 1.0, 0.5, -10.0, 0.001];
        for &v in &values {
            let encoded = to_8_24(v);
            let decoded = from_8_24(encoded);
            assert!((v - decoded).abs() < 0.0001, "failed for {v}");
        }
    }

    #[test]
    fn byte_encode_decode() {
        let val = PropertyValue::Byte(42);
        let encoded = val.encode();
        let decoded = PropertyValue::decode(PropertyType::Byte, &encoded).unwrap();
        assert_eq!(val, decoded);
    }

    #[test]
    fn string_null_terminated() {
        let val = PropertyValue::String("Mic 1".to_string());
        let encoded = val.encode();
        assert_eq!(encoded.len(), 32);
        assert_eq!(encoded[5], 0);
        let decoded = PropertyValue::decode(PropertyType::String, &encoded).unwrap();
        assert_eq!(val, decoded);
    }
}

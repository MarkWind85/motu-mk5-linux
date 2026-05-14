/// MOTU mk5 MIDI SysEx transport layer.
///
/// Implements the binary framing protocol extracted from CueMix5:
///   Header: F0 00 00 3B 00 01
///   Request byte: direction_bit(6) | request_id
///   Payload: 7-bit MIDI SysEx encoding
///   Footer: F7

const SYSEX_END: u8 = 0xF7;
const MOTU_HEADER: [u8; 6] = [0xF0, 0x00, 0x00, 0x3B, 0x00, 0x01];

const DIRECTION_HOST_TO_DEVICE: u8 = 0;
const DIRECTION_BIT: u8 = 6;
const DIRECTION_MASK: u8 = 1 << DIRECTION_BIT;
const REQUEST_ID_MASK: u8 = (!DIRECTION_MASK) & 0x7F;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestId {
    SetProperty,
    ProtocolProbe,
    EnableSysexApi,
}

impl RequestId {
    fn as_u8(self) -> u8 {
        match self {
            RequestId::SetProperty => 0,
            RequestId::ProtocolProbe => 1,
            RequestId::EnableSysexApi => 2,
        }
    }

    fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(RequestId::SetProperty),
            1 => Some(RequestId::ProtocolProbe),
            2 => Some(RequestId::EnableSysexApi),
            _ => None,
        }
    }
}

fn make_request_byte(direction: u8, id: RequestId) -> u8 {
    (direction << DIRECTION_BIT) | (id.as_u8() & REQUEST_ID_MASK)
}

/// Encode raw bytes into 7-bit MIDI SysEx encoding.
/// Every 7 data bytes get a leading byte containing their MSBs.
fn encode_7bit(data: &[u8]) -> Vec<u8> {
    let extra = (data.len() + 6) / 7;
    let mut out = Vec::with_capacity(data.len() + extra);

    let mut i = 0;
    while i < data.len() {
        let chunk_len = (data.len() - i).min(7);
        let mut msb_byte: u8 = 0;
        for j in 0..chunk_len {
            let shift = 7 - j;
            msb_byte |= ((data[i + j] & 0x80) >> shift) as u8;
        }
        out.push(msb_byte);
        for j in 0..chunk_len {
            out.push(data[i + j] & 0x7F);
        }
        i += chunk_len;
    }

    out
}

/// Decode 7-bit MIDI SysEx encoding back to raw bytes.
fn decode_7bit(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;

    while i < data.len() {
        let msb_byte = data[i];
        i += 1;
        let mut shift = 7u8;
        while i < data.len() && shift > 0 {
            let msb = ((msb_byte << shift) & 0x80) as u8;
            out.push(msb | data[i]);
            i += 1;
            shift -= 1;
        }
    }

    out
}

/// Build a complete SysEx message for sending to the device.
pub fn build_message(request: RequestId, payload: &[u8]) -> Vec<u8> {
    let encoded = encode_7bit(payload);
    let mut msg = Vec::with_capacity(MOTU_HEADER.len() + 1 + encoded.len() + 1);
    msg.extend_from_slice(&MOTU_HEADER);
    msg.push(make_request_byte(DIRECTION_HOST_TO_DEVICE, request));
    msg.extend_from_slice(&encoded);
    msg.push(SYSEX_END);
    msg
}

/// Build a protocol probe message (no payload).
pub fn build_probe() -> Vec<u8> {
    let mut msg = Vec::with_capacity(8);
    msg.extend_from_slice(&MOTU_HEADER);
    msg.push(make_request_byte(DIRECTION_HOST_TO_DEVICE, RequestId::ProtocolProbe));
    msg.push(SYSEX_END);
    msg
}

/// Build an enable SysEx API message (no payload).
pub fn build_enable_api() -> Vec<u8> {
    let mut msg = Vec::with_capacity(8);
    msg.extend_from_slice(&MOTU_HEADER);
    msg.push(make_request_byte(DIRECTION_HOST_TO_DEVICE, RequestId::EnableSysexApi));
    msg.push(SYSEX_END);
    msg
}

/// Build a property set message.
pub fn build_set_property(prop_id: u16, index: u16, data: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(6 + data.len());
    payload.extend_from_slice(&prop_id.to_be_bytes());
    payload.extend_from_slice(&index.to_be_bytes());
    payload.extend_from_slice(&(data.len() as u16).to_be_bytes());
    payload.extend_from_slice(data);
    build_message(RequestId::SetProperty, &payload)
}

#[derive(Debug, Clone)]
pub struct ParsedMessage {
    pub request_id: RequestId,
    pub direction: u8,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PropertyMessage {
    pub prop_id: u16,
    pub index: u16,
    pub data: Vec<u8>,
}

/// Parse an incoming SysEx message from the device.
pub fn parse_message(raw: &[u8]) -> Option<ParsedMessage> {
    if raw.len() < 8 {
        return None;
    }
    if raw[..6] != MOTU_HEADER {
        return None;
    }
    if raw[raw.len() - 1] != SYSEX_END {
        return None;
    }

    let request_byte = raw[6];
    let direction = (request_byte & DIRECTION_MASK) >> DIRECTION_BIT;
    let request_id = RequestId::from_u8(request_byte & REQUEST_ID_MASK)?;

    let encoded_payload = &raw[7..raw.len() - 1];
    let payload = if encoded_payload.is_empty() {
        Vec::new()
    } else {
        decode_7bit(encoded_payload)
    };

    Some(ParsedMessage {
        request_id,
        direction,
        payload,
    })
}

/// Extract property ID, index, and data from a SetProperty payload.
pub fn parse_property(payload: &[u8]) -> Option<PropertyMessage> {
    if payload.len() < 4 {
        return None;
    }

    let prop_id = u16::from_be_bytes([payload[0], payload[1]]);
    let index = u16::from_be_bytes([payload[2], payload[3]]);
    let data = payload[4..].to_vec();

    Some(PropertyMessage {
        prop_id,
        index,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_7bit_roundtrip() {
        let data: Vec<u8> = (0..=255).collect();
        let encoded = encode_7bit(&data);
        let decoded = decode_7bit(&encoded);
        assert_eq!(data, decoded);
    }

    #[test]
    fn probe_message_format() {
        let msg = build_probe();
        assert_eq!(msg[0], 0xF0);
        assert_eq!(&msg[1..4], &[0x00, 0x00, 0x3B]);
        assert_eq!(msg[4], 0x00); // protocol ID
        assert_eq!(msg[5], 0x01); // protocol ID
        assert_eq!(msg[6], make_request_byte(DIRECTION_HOST_TO_DEVICE, RequestId::ProtocolProbe));
        assert_eq!(*msg.last().unwrap(), SYSEX_END);
    }

    #[test]
    fn set_property_send_encodes_correctly() {
        let msg = build_set_property(1016, 5, &[0x00, 0x80, 0x00, 0x00]);
        let parsed = parse_message(&msg).unwrap();
        assert_eq!(parsed.request_id, RequestId::SetProperty);
        // Host→device includes length field: [prop_id(2), index(2), length(2), data(N)]
        assert_eq!(parsed.payload[0..2], 1016u16.to_be_bytes());
        assert_eq!(parsed.payload[2..4], 5u16.to_be_bytes());
        assert_eq!(parsed.payload[4..6], 4u16.to_be_bytes());
        assert_eq!(&parsed.payload[6..], &[0x00, 0x80, 0x00, 0x00]);
    }

    #[test]
    fn parse_device_property_message() {
        // Device→host format: [prop_id(2), index(2), data(N)] — no length field
        let payload = vec![0x03, 0xF8, 0x00, 0x05, 0x00, 0x80, 0x00, 0x00];
        let prop = parse_property(&payload).unwrap();
        assert_eq!(prop.prop_id, 1016);
        assert_eq!(prop.index, 5);
        assert_eq!(prop.data, vec![0x00, 0x80, 0x00, 0x00]);
    }
}

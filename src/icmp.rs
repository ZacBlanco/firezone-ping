//! Contains implementation for generating ICMP packets

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum IcmpType {
    EchoReply,
    EchoRequest,
    Unknown(u8),
}

impl From<u8> for IcmpType {
    fn from(value: u8) -> Self {
        match value {
            0 => IcmpType::EchoReply,
            8 => IcmpType::EchoRequest,
            _ => IcmpType::Unknown(value),
        }
    }
}

impl From<IcmpType> for u8 {
    fn from(value: IcmpType) -> Self {
        match value {
            IcmpType::EchoReply => 0,
            IcmpType::EchoRequest => 8,
            IcmpType::Unknown(x) => x,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum IcmpCode {
    Zero,
    Unknown(u8),
}

impl From<u8> for IcmpCode {
    fn from(value: u8) -> Self {
        match value {
            0 => IcmpCode::Zero,
            _ => IcmpCode::Unknown(value),
        }
    }
}

impl From<IcmpCode> for u8 {
    fn from(value: IcmpCode) -> Self {
        match value {
            IcmpCode::Zero => 0,
            IcmpCode::Unknown(x) => x,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IcmpEcho {
    ty: IcmpType,
    code: IcmpCode,
    id: u16,
    seq: u16,
}

impl From<IcmpEcho> for Vec<u8> {
    fn from(value: IcmpEcho) -> Vec<u8> {
        let mut buf = vec![0; 8];
        value.construct_buf(&mut buf);
        buf
    }
}

impl From<Vec<u8>> for IcmpEcho {
    fn from(value: Vec<u8>) -> Self {
        dbg!(&value[4..6]);
        IcmpEcho {
            ty: u8::from_be(value[0]).into(),
            code: u8::from_be(value[1]).into(),
            id: u16::from_be_bytes(value[4..6].try_into().unwrap()),
            seq: u16::from_be_bytes(value[6..8].try_into().unwrap()),
        }
    }
}

impl IcmpEcho {
    pub fn new(id: u16, seq: u16) -> Self {
        IcmpEcho {
            ty: IcmpType::EchoRequest,
            code: IcmpCode::Zero,
            id,
            seq,
        }
    }

    pub fn construct_buf(&self, buf: &mut [u8]) {
        buf[0] = (u8::from(self.ty)).to_be_bytes()[0];
        buf[1] = (u8::from(self.code)).to_be_bytes()[0];
        buf[2..4].copy_from_slice(&self.checksum().to_be_bytes());
        buf[4..6].copy_from_slice(&self.id.to_be_bytes());
        buf[6..8].copy_from_slice(&self.seq.to_be_bytes());
    }

    pub fn size() -> usize {
        std::mem::size_of::<Self>()
    }

    /// ICMP checksum
    /// doesn't handle odd-length packets
    /// length of the packets generated in the current implementation should
    /// always be even.
    pub fn checksum(&self) -> u16 {
        let word1 = ((u8::from(self.ty) as u16) << 8) + (u8::from(self.code) as u16);
        let items = vec![word1, self.id, self.seq];
        let mut sum = 0u16;
        for item in items {
            sum = sum.wrapping_add(item);
        }
        !sum
    }
}

#[cfg(test)]
mod test {
    use crate::icmp::IcmpEcho;

    use super::IcmpCode;

    #[test]
    fn csum() {
        assert_eq!(
            IcmpEcho {
                ty: super::IcmpType::EchoRequest,
                code: IcmpCode::Zero,
                id: 12345,
                seq: 54321,
            }
            .checksum(),
            62357
        );
    }

    #[test]
    fn size() {
        assert_eq!(8, std::mem::size_of::<IcmpEcho>());
    }
}

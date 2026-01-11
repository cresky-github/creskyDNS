use anyhow::Result;
use hickory_proto::op::Message;
use hickory_proto::serialize::binary::BinEncodable;

pub fn encode_dns(msg: &Message) -> Result<Vec<u8>> {
    Ok(msg.to_bytes()?)
}

pub fn parse_dns(buf: &[u8]) -> Result<Message> {
    Ok(Message::from_vec(buf)?)
}

pub fn get_qname(msg: &Message) -> Option<String> {
    msg.queries().first().map(|q| q.name().to_utf8())
}
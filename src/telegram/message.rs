use chrono::{DateTime, Utc};
use grammers_client::types as gr_types;
use grammers_tl_types::enums;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Peer {
    User(i64),
    Chat(i64),
    Channel(i64),
}

impl Peer {
    pub fn from_enum(p: enums::Peer) -> Self {
        match p {
            enums::Peer::Chat(peer) => Peer::Chat(peer.chat_id),
            enums::Peer::User(peer) => Peer::User(peer.user_id),
            enums::Peer::Channel(peer) => Peer::Channel(peer.channel_id),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageFwdHeader {
    pub imported: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_id: Option<Peer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_name: Option<String>,
    pub date: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_post: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_from_peer: Option<Peer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_from_msg_id: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub psa_type: Option<String>,
}

impl MessageFwdHeader {
    fn parse(hearer: enums::MessageFwdHeader) -> Self {
        match hearer {
            enums::MessageFwdHeader::Header(h) => MessageFwdHeader {
                imported: h.imported,
                from_id: h.from_id.map(Peer::from_enum),
                from_name: h.from_name,
                date: h.date,
                channel_post: h.channel_post,
                post_author: h.post_author,
                saved_from_peer: h.saved_from_peer.map(Peer::from_enum),
                saved_from_msg_id: h.saved_from_msg_id,
                psa_type: h.psa_type,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageReplyHeader {
    #[serde(default, skip_serializing_if = "is_false")]
    pub reply_to_scheduled: bool,
    pub reply_to_msg_id: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to_peer_id: Option<Peer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to_top_id: Option<i32>,
}

impl MessageReplyHeader {
    fn parse(repl: enums::MessageReplyHeader) -> Self {
        match repl {
            enums::MessageReplyHeader::Header(h) => MessageReplyHeader {
                reply_to_scheduled: h.reply_to_scheduled,
                reply_to_msg_id: h.reply_to_msg_id,
                reply_to_peer_id: h.reply_to_peer_id.map(Peer::from_enum),
                reply_to_top_id: h.reply_to_top_id,
            },
        }
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageReplies {
    #[serde(default, skip_serializing_if = "is_false")]
    pub comments: bool,
    pub replies: i32,
    pub replies_pts: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recent_repliers: Option<Vec<Peer>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_id: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_max_id: Option<i32>,
}

impl MessageReplies {
    fn parse(repl: enums::MessageReplies) -> Self {
        match repl {
            enums::MessageReplies::Replies(r) => MessageReplies {
                comments: r.comments,
                replies: r.replies,
                replies_pts: r.replies_pts,
                recent_repliers: r
                    .recent_repliers
                    .map(|v| v.into_iter().map(Peer::from_enum).collect()),
                channel_id: r.channel_id,
                max_id: r.max_id,
                read_max_id: r.read_max_id,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Sender {
    User(String),
    Group(String),
    Channel(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: i32,
    pub text: String,
    pub date: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub out: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub mentioned: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub media_unread: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub silent: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub post: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub from_scheduled: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub edit_hide: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub pinned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender: Option<Sender>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forward_header: Option<MessageFwdHeader>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via_bot_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<MessageReplyHeader>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forward_count: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_count: Option<MessageReplies>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit_date: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grouped_id: Option<i64>,
    // media: Option<Media>,
}

impl Message {
    pub fn parse(msg: gr_types::Message) -> Self {
        Message {
            id: msg.id(),
            text: msg.text().to_string(),
            date: msg.date(),
            out: msg.outgoing(),
            mentioned: msg.mentioned(),
            media_unread: msg.media_unread(),
            silent: msg.silent(),
            post: msg.post(),
            from_scheduled: msg.from_scheduled(),
            edit_hide: msg.edit_hide(),
            pinned: msg.pinned(),
            sender: msg.sender().map(parse_sender),
            forward_header: msg.forward_header().map(MessageFwdHeader::parse),
            via_bot_id: msg.via_bot_id(),
            reply_to: msg.reply_header().map(MessageReplyHeader::parse),
            forward_count: msg.forward_count(),
            reply_count: msg.reply_count().map(MessageReplies::parse),
            edit_date: msg.edit_date(),
            post_author: msg.post_author().map(String::from),
            grouped_id: msg.grouped_id(),
        }
    }
}

fn parse_sender(s: gr_types::Chat) -> Sender {
    match s {
        gr_types::Chat::User(u) => Sender::User(u.full_name()),
        gr_types::Chat::Group(g) => Sender::Group(g.title().to_string()),
        gr_types::Chat::Channel(c) => Sender::Channel(c.title().to_string()),
    }
}

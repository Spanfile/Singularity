use crate::error::{EvhError, EvhResult};
use log::*;
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt, BufStream},
    net::UnixStream,
};

#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum RecControlMessage<'a> {
    Ping,
    Version,
    GetAll,
    Get(&'a str),
    ReloadLuaScript(Option<&'a Path>),
}

pub struct RecControl {
    control_socket: PathBuf,
}

pub struct RecursorVersion(pub String);

impl RecControl {
    pub async fn new<P>(socket_path: P) -> EvhResult<(Self, RecursorVersion)>
    where
        P: Into<PathBuf>,
    {
        let inst = Self {
            control_socket: socket_path.into(),
        };

        let version = inst.send_control_message(RecControlMessage::Version).await?;
        let version = RecursorVersion(version);

        Ok((inst, version))
    }

    pub async fn send_control_message(&self, message: RecControlMessage<'_>) -> EvhResult<String> {
        // in rec_control, the communication is done based on an answer struct. the same answer struct is used in both
        // directions and contains the following:
        // - the operation's return code. in case the answer is the initial message sent to the recursor, the code is 0
        // - the operation result as a string. in case the answer is the initial message, it is the command to send

        // the communication flow, based on https://github.com/PowerDNS/pdns/blob/master/pdns/rec_channel.cc
        // - send the answer's return code over the channel
        // - send the answer's result string's length
        // - send the result string
        // all numbers sent over the channel seem to be little endian

        // there's a mechanism to send file descriptors as well, TODO: implement it? it's required for things like
        // dumping the cache

        let control_stream = UnixStream::connect(&self.control_socket).await?;
        let mut control_stream = BufStream::new(control_stream);

        debug!("Sending Recursor control message: {:?}", message);

        send_message(&mut control_stream, message).await?;
        let resp = recv_message(&mut control_stream).await?;

        debug!("Recursor control message response: {}", resp);
        Ok(resp)
    }
}

async fn send_message<W>(sock: &mut W, message: RecControlMessage<'_>) -> EvhResult<()>
where
    W: io::AsyncWrite + Unpin,
{
    let msg = message.as_string();
    let bytes = msg.as_bytes();

    sock.write_i32_le(0).await?; // send the return code, 0 in every case we're sending data. it's probably 32 bits long?
    sock.write_u64_le(msg.len() as u64).await?; // send the message's length
    sock.write_all(bytes).await?; // send the message
    sock.flush().await?;

    Ok(())
}

async fn recv_message<R>(sock: &mut R) -> EvhResult<String>
where
    R: io::AsyncRead + Unpin,
{
    let ret = sock.read_i32_le().await?;
    let len = sock.read_u64_le().await? as usize;

    debug!("rec_control: ret {}, len {}", ret, len);

    let mut buf = vec![0u8; len];
    sock.read_exact(&mut buf).await?;

    // the last byte in each response is a newline, get rid of it
    buf.truncate(buf.len() - 1);

    let resp = String::from_utf8(buf).map_err(|_| EvhError::TextNotUtf8)?;

    if ret == 0 {
        Ok(resp)
    } else {
        Err(EvhError::RecControl(ret, resp))
    }
}

impl<'a> RecControlMessage<'a> {
    fn as_string(self) -> Cow<'a, str> {
        match self {
            RecControlMessage::Ping => Cow::Borrowed("ping"),
            RecControlMessage::Version => Cow::Borrowed("version"),
            RecControlMessage::GetAll => Cow::Borrowed("get-all"),
            RecControlMessage::Get(param) => Cow::Owned(format!("get {}", param)),
            RecControlMessage::ReloadLuaScript(None) => Cow::Borrowed("reload-lua-script"),
            RecControlMessage::ReloadLuaScript(Some(filename)) => {
                Cow::Owned(format!("reload-lua-script {}", filename.display()))
            }
        }
    }
}

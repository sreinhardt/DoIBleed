use std::io::prelude::*;
use std::io::Cursor;
use std::io::BufRead;
use std::time::{Instant, Duration};
use std::str;
use tokio::net::{TcpStream};
use tokio::prelude::*;
use futures::Poll;
use state_machine_future::RentToOwn;
use mio::Ready;
use rand::{Rng, thread_rng};
use rand::seq;
use bytes::BytesMut;

use super::error::*;

pub const MAX_LVL: usize = 10;
pub const MSG_SZ: usize = 100;
#[cfg(feature = "real_flag")] static FLAG: &'static str = "40ByteCTF{1_c@n-h4z=Th3_m3M0rI3s?}";
#[cfg(not(feature = "real_flag"))] static FLAG: &'static str = "40ByteCTF{Not actually the real flag}";
pub static INTRO: &str = "Welcome to DoIBleed\nComplete the problem at hand, send a [space] when ready...\n";

#[derive(StateMachineFuture)]
pub enum DoIBleed {
  #[state_machine_future(start, transitions(StartRead))]
  Start {
    stream: TcpStream,
    id: usize,
  },
  #[state_machine_future(transitions(PlayingWrite))]
  StartRead {
    stream: TcpStream,
    id: usize,
  },
  #[state_machine_future(transitions(PlayingRead, Failed))]
  PlayingWrite {
    stream: TcpStream,
    id: usize,
    time: Instant,
    lvl_count: usize,
    level: Level,
    buff: Message,
  },
  #[state_machine_future(transitions(PlayingWrite, Failed, Finished))]
  PlayingRead {
    stream: TcpStream,
    id: usize,
    time: Instant,
    lvl_count: usize,
    level: Level,
    buff: Message,
  },
  #[state_machine_future(transitions(Finished))]
  Failed {
    stream: TcpStream,
    id: usize,
    lvl: usize,
    reason: Reason,
  },
  #[state_machine_future(ready)]
  Finished(()),
  #[state_machine_future(error)]
  Error(DoIBleedError)
}

impl PollDoIBleed for DoIBleed {
  fn poll_start<'a>(start: &'a mut RentToOwn<'a, Start>)
    -> Poll<AfterStart, DoIBleedError>
  {
    //trace!{"DoIBleed::poll_start({})", start.id};
    // ensure we can write, or return early
    try_ready!{start.stream.poll_write_ready()};
    let intro = Message::str_to_bytes(INTRO);
    let mut intro = Cursor::new(intro);
    let size = try_ready!{start.stream.write_buf(&mut intro)};
    debug!{"Wrote {:?} bytes: {}", size, start.id};
    let start = start.take();
    let read = StartRead {
      stream: start.stream,
      id: start.id,
    };
    transition!{read}
  }

  fn poll_start_read<'a>(start: &'a mut RentToOwn<'a, StartRead>)
    -> Poll<AfterStartRead, DoIBleedError>
  {
    //trace!{"DoIBleed::poll_start_read({})", start.id};
    // catch the read here, indicating acceptance
    try_ready!{start.stream.poll_read_ready(Ready::readable())};

    let mut buf = BytesMut::with_capacity(MSG_SZ+1);
    let size = try_ready!{start.stream.read_buf(&mut buf)};
    debug!{"Client sent {:?} bytes: {}", size, start.id};
    // read into Message, leave anything after 0xa in prior
    let msg: Message = Message::from_bytes(&mut buf)?;
    let start = start.take();
    let playing = PlayingWrite {
      stream: start.stream,
      id: start.id,
      time: Instant::now(),
      lvl_count: 1,
      level: Level::new(),
      buff: msg,
    };
    transition!{playing}
  }

  fn poll_playing_write<'a>(playing: &'a mut RentToOwn<'a, PlayingWrite>)
    -> Poll<AfterPlayingWrite, DoIBleedError>
  {
    //trace!{"DoIBleed::poll_playing({}) level = {}", playing.id, playing.lvl_count};
    let timeout = MAX_LVL+1-playing.lvl_count;
    let timeout = Duration::new(timeout as u64, 0);
    let taken = Instant::now() - playing.time;
    if timeout < taken {
      let failed = playing.take();
      let failed = Failed {
        stream: failed.stream,
        id: failed.id,
        lvl: failed.lvl_count,
        reason: Reason::Timeout,
      };
      transition!{failed}
    }
    try_ready!{playing.stream.poll_write_ready()};
    let level = playing.level.to_string();
    let level = format!{"Welcome to level {}\nSolve this problem:\n{} ", playing.lvl_count, level};
    let mut level = Message::str_to_bytes(&level);
    Message::extend_with_flag(&mut level, playing.buff.size as usize, playing.lvl_count);
    let size = try_ready!{playing.stream.write_buf(&mut Cursor::new(level))};
    debug!{"Wrote {:?} bytes to client: {}", size, playing.id};
    let playing = playing.take();
    let p2 = PlayingRead {
      stream: playing.stream,
      id: playing.id,
      time: Instant::now(), // set time to once sent
      lvl_count: playing.lvl_count,
      level: playing.level,
      buff: playing.buff,
    };
    transition!{p2}
  }
  fn poll_playing_read<'a>(playing: &'a mut RentToOwn<'a, PlayingRead>)
    -> Poll<AfterPlayingRead, DoIBleedError>
  {
    //trace!{"DoIBleed::poll_playing_read({}) level = {}", playing.id, playing.lvl_count};
    let timeout = MAX_LVL+1-playing.lvl_count;
    let timeout = Duration::new(timeout as u64, 0);
    let taken = Instant::now() - playing.time;
    if timeout < taken {
      let failed = playing.take();
      let failed = Failed {
        stream: failed.stream,
        id: failed.id,
        lvl: failed.lvl_count,
        reason: Reason::Timeout,
      };
      transition!{failed}
    }
    try_ready!{playing.stream.poll_read_ready(Ready::readable())};
    let mut buf = BytesMut::with_capacity(MSG_SZ+1);
    let size = try_ready!{playing.stream.read_buf(&mut buf)};
    debug!{"Read {:?} bytes from client: {}", size, playing.id};
    let msg: Message = Message::from_bytes(&mut buf)?;
    let mut playing = playing.take();
    if !playing.level.validate_answer(&msg) {
      let failed = Failed {
        stream: playing.stream,
        id: playing.id,
        lvl: playing.lvl_count,
        reason: Reason::WrongAnswer,
      };
      transition!{failed}
    } else if playing.lvl_count < MAX_LVL {
      let p2 = PlayingWrite {
        stream: playing.stream,
        id: playing.id,
        time: playing.time,
        lvl_count: playing.lvl_count + 1,
        level: Level::new(),
        buff: msg,
      };
      transition!{p2}
    } else { // correct and finished
      let msg = Message::str_to_bytes("You Won!!");
      try_ready!{playing.stream.poll_write(&msg)};
      transition!{Finished(())}
    }
  }
  fn poll_failed<'a>(failed: &'a mut RentToOwn<'a, Failed>)
    -> Poll<AfterFailed, DoIBleedError>
  {
    //trace!{"DoIBleed::poll_failed({}) level = {}", failed.id, failed.lvl};
    try_ready!{failed.stream.poll_write_ready()};
    let msg = failed.reason.to_str();
    let msg = Message::str_to_bytes(&msg);
    try_ready!{failed.stream.poll_write(&msg)};
    transition!{Finished(())}
  }
}

pub struct Message {
  size: u8,        // size to print, NOT size of buff
  inner: Vec<u8>,  // total size available
}
impl Message {
  fn new() -> Self {
    Message {
      size: 0,
      inner: Vec::with_capacity(MSG_SZ),
    }
  }
  fn inner(&self) -> &Vec<u8> {
    &self.inner
  }
  fn str_to_bytes(msg: &str) -> Vec<u8> {
    let len = msg.len() as u8;
    let mut vec = Vec::with_capacity(msg.len()+1);
    vec.push(len);
    vec.extend_from_slice(&mut msg.as_bytes());
    vec
  }
  fn extend_with_flag(buf: &mut Vec<u8>, size: usize, lvl: usize) {
    if buf.len() < size {
      let remaining = size - buf.len();
      let mut rng = thread_rng();
      let mut rand_vec: Vec<u8> = seq::sample_iter(&mut rng, 0x20..0x7F, remaining).unwrap();
      buf.append(&mut rand_vec);
      let fsize = FLAG.len() / (MAX_LVL + 1 - lvl);  // percentage of flag to print (11 means we never hit 0)
      if MSG_SZ - fsize < size {                     // total - size of flag < size == should display flag
        debug!{"Writing portion of flag"};
        let fsize = size - (MSG_SZ - fsize);         // requested - start of flag = actual portion to print
        let flag = FLAG[0..fsize].as_bytes();        // slice to print
        let idx = buf.len() - fsize;                 // idx to end of buffer, beginning of printed FLAG
        for (i,v) in buf.iter_mut().enumerate() {
          if idx <= i {
            *v = flag[i-idx];
          }
        }
      }
    }
  }
  fn from_bytes(bytes: &mut BytesMut/*, client: usize*/) -> DoIBleedResult<Message> {
    // grab first 2 bytes for transmute into u16 of client msg size, but dont really use that
    let mut bytes = Cursor::new(bytes);
    let mut size: [u8; 1] = [0; 1];
    bytes.read(&mut size)?;
    let size: u8 = size[0];
    if MSG_SZ < size as usize {
      return Err(DoIBleedError::MsgTooLarge(size as usize))
    }
    // find 0x0A in remaining data, this is what we take for inner
    let mut msg = Message::new();
    msg.size = size;
    let _read = bytes.read_until(b'\n', &mut msg.inner)?;
    //debug!{"Client({}) passed msg: {:?}", client, msg.inner};
    Ok(msg)
  }
}
pub struct Level {
  left: i16,
  right: i16,
  operator: u8,
  result: i64,
}
impl Level {
  fn new() -> Level {
    let mut rng = thread_rng();
    let (left, right) = rng.gen::<(i16, i16)>();
    let result: i64;
    let oper = match rng.gen_range::<u8>(0, 4) {
      0 => { result = (left as i64) + (right as i64); '+' as u8 },
      1 => { result = (left as i64) - (right as i64); '-' as u8 },
      2 => { result = (left as i64) * (right as i64); '*' as u8 },
      3 => { result = (left as i64) / (right as i64); '/' as u8 },
      _ => { result = (left as i64) % (right as i64); '%' as u8 },
    };
    debug!{"Level generated: {} {} {} = {}", left, oper as char, right, result};
    Level {
      left: left,
      right: right,
      operator: oper,
      result: result,
    }
  }
  fn to_string(&self) -> String {
    let s = format!{"{} {} {} = ", self.left, self.operator as char, self.right};
    s.to_string()
  }
  fn validate_answer(&self, msg: &Message) -> bool {
    let msg = msg.inner();
    let msg = match str::from_utf8(msg) {
      Ok(val) => val,
      Err(_) => {
        warn!{"Client failed to parse from utf-8"};
        return false
      },
    };
    let len = msg.len();
    if len > 20 {
      warn!{"Client sent too long a string"}
      return false;
    }
    let val: i64 = match msg.parse() {
      Ok(val) => val,
      Err(_) => {
        warn!{"Client response failed to parse"};
        return false
      },
    };
    self.result == val
  }
}

pub enum Reason {
  Timeout,
  WrongAnswer,
}
impl Reason {
  fn to_str(&self) -> &'static str {
    match *self {
      Reason::Timeout => "Timeout occurred, exiting",
      Reason::WrongAnswer => "Wrong answer provided, exiting",
    }
  }
}
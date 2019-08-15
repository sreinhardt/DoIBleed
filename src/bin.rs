#[macro_use] extern crate log;
extern crate pretty_env_logger;
extern crate rand;
extern crate tokio;
extern crate tokio_core;
#[macro_use] extern crate futures;
#[macro_use] extern crate state_machine_future;
extern crate bytes;
extern crate mio;

mod error;
mod game;

use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::net::TcpListener;
use tokio::prelude::*;
use tokio_core::reactor::Core;

use game::*;

#[cfg(feature = "real_flag")]      const IP: &str = "0.0.0.0";
#[cfg(not(feature = "real_flag"))] const IP: &str = "127.0.0.1";
const PORT: &str = "22380"; // rax2 =16 140160 (heartbleed cve-2014-0160)

fn main() {
  pretty_env_logger::init();

  debug!{"Starting Tokio::Core"}
  let mut core = Core::new().unwrap();
  let handle = core.handle();

  trace!{"Preparing server configuration"}
  let addr = format!{"{}:{}", IP, PORT};
  debug!{"Binding to: {}", addr};
  let addr = addr.parse().unwrap();
  let listener = TcpListener::bind(&addr).unwrap();
  let client_count = AtomicUsize::new(0);
  let server = listener.incoming().for_each(move |socket| { // TcpStream
      debug!{"Accepted socket; addr={:?}", socket.peer_addr().unwrap()};
      // Process socket here.
      let id = client_count.fetch_add(1, Ordering::SeqCst);
      let game = DoIBleed::start(socket, id)
        .map_err(|err| { error!("Client error = {:?}", err); });

      handle.spawn(game);
      Ok(())
  })
  .map_err(|err| { error!("accept error = {:?}", err); });

  core.run(server).unwrap();
}
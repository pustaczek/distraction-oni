use crate::config::Config;
use crate::slots::Slots;
use chrono::{DateTime, Utc};
use std::time::Duration;

mod config;
mod slots;
mod sources;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Activity {
	Website { domain: String },
}

#[derive(Debug)]
pub struct Event {
	activity: Activity,
	timestamp: DateTime<Utc>,
	is_active: bool,
}

fn main() {
	let config = Config::load();
	sources::webext::proxy::check_and_run(config.sources.webext.port);
	let mut slots = Slots::new();
	let mut conn = sources::webext::WebExt::new(config.sources.webext.port);
	loop {
		let event = conn.next_timeout(Duration::from_secs(1));
		if let Some(event) = event {
			println!("{:?}", event);
			slots.process_event(event);
		}
	}
}

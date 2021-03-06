mod serde_duration;
mod serde_time;

use chrono::{DateTime, Local, NaiveTime};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct General {
	pub prevent_browser_close: bool,
	pub close_all_on_block: bool,
	#[serde(default, deserialize_with = "serde_duration::deserialize_option")]
	pub close_all_after_block: Option<Duration>,
}

#[derive(Debug, Deserialize)]
pub struct Category {
	#[serde(default)]
	pub domains: Vec<String>,
	#[serde(default)]
	pub subreddits: Vec<String>,
	#[serde(default)]
	pub githubs: Vec<String>,
	#[serde(default)]
	pub regexes: Vec<String>,
	#[serde(default)]
	pub processes: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct TimeRange {
	#[serde(with = "serde_time")]
	since: NaiveTime,
	#[serde(with = "serde_time")]
	until: NaiveTime,
}

#[derive(Debug, Deserialize)]
pub struct Rule {
	#[serde(default)]
	pub allowed: Option<TimeRange>,
	pub categories: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct PermitLength {
	#[serde(default, deserialize_with = "serde_duration::deserialize_option")]
	pub default: Option<Duration>,
	#[serde(default, deserialize_with = "serde_duration::deserialize_option")]
	pub maximum: Option<Duration>,
}

#[derive(Debug, Deserialize)]
pub struct Permit {
	#[serde(default)]
	pub length: PermitLength,
	#[serde(default, deserialize_with = "serde_duration::deserialize_option")]
	pub cooldown: Option<Duration>,
	#[serde(default)]
	pub available: Option<TimeRange>,
	pub categories: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
	pub general: General,
	pub category: HashMap<String, Category>,
	pub rule: HashMap<String, Rule>,
	#[serde(default)]
	pub permit: HashMap<String, Permit>,
}

impl Config {
	pub fn load() -> Config {
		let path = dirs::config_dir().unwrap().join("vaxtify.toml");
		let file = std::fs::read_to_string(path).unwrap();
		let config = Config::parse(&file);
		assert!(!config.general.prevent_browser_close || !config.general.close_all_on_block);
		assert!(config.general.close_all_on_block || config.general.close_all_after_block.is_none());
		config
	}

	pub fn parse(file: &str) -> Config {
		toml::from_str(file).unwrap()
	}
}

impl TimeRange {
	fn contains(&self, now: &DateTime<Local>) -> bool {
		let TimeRange { since, until } = *self;
		let time = now.naive_local().time();
		if since <= until {
			time >= since && time < until
		} else {
			time >= since || time < until
		}
	}
}

impl Rule {
	pub fn is_active(&self, now: &DateTime<Local>) -> bool {
		self.allowed.map_or(true, |allowed| !allowed.contains(now))
	}

	pub fn next_change_time(&self, now: &DateTime<Local>) -> Option<DateTime<Local>> {
		let TimeRange { since, until } = self.allowed?;
		let next_start = upper_bound_with_time(now, &since);
		let next_end = upper_bound_with_time(now, &until);
		Some(next_start.min(next_end))
	}
}

impl Permit {
	pub fn is_available(&self, now: &DateTime<Local>) -> bool {
		self.available.map_or(true, |available| available.contains(now))
	}
}

fn upper_bound_with_time(greater_than: &DateTime<Local>, set_time: &NaiveTime) -> DateTime<Local> {
	let mut candidate = greater_than.date();
	while candidate.and_time(*set_time).unwrap() <= *greater_than {
		candidate = candidate.succ();
	}
	candidate.and_time(*set_time).unwrap()
}

#[test]
fn example() {
	let text = r#"
[general]
prevent_browser_close = false
close_all_on_block = true
close_all_after_block = { mins = 5 }

[category.example]
domains = ["example.com"]
subreddits = ["all"]
githubs = ["pustaczek/icie"]
regexes = ["example\\.org"]
processes = ["chrome"]

[category.other]
githubs = ["pustaczek/vaxtify"]

[rule.things]
allowed.since = { hour = 23, min = 0 }
allowed.until = { hour = 0, min = 0 }
categories = ["example"]

[rule.never]
categories = ["other"]

[permit.example]
length.default = { mins = 30 }
length.maximum = { mins = 40 }
cooldown = { hours = 20 }
available.since = { hour = 20, min = 0 }
available.until = { hour = 0, min = 0 }
categories = ["other"]
"#;
	let config = Config::parse(text);
	assert_eq!(config.general.prevent_browser_close, false);
	assert_eq!(config.general.close_all_on_block, true);
	assert_eq!(config.general.close_all_after_block, Some(Duration::from_secs(5 * 60)));
	assert_eq!(config.category.len(), 2);
	assert_eq!(config.category["example"].domains, ["example.com"]);
	assert_eq!(config.category["example"].subreddits, ["all"]);
	assert_eq!(config.category["example"].githubs, ["pustaczek/icie"]);
	assert_eq!(config.category["example"].regexes, ["example\\.org"]);
	assert_eq!(config.category["example"].processes, ["chrome"]);
	assert_eq!(config.category["other"].githubs, ["pustaczek/vaxtify"]);
	assert_eq!(config.rule.len(), 2);
	assert_eq!(
		config.rule["things"].allowed,
		Some(TimeRange { since: NaiveTime::from_hms(23, 0, 0), until: NaiveTime::from_hms(0, 0, 0) })
	);
	assert_eq!(config.rule["things"].categories, ["example"]);
	assert_eq!(config.rule["never"].allowed, None);
	assert_eq!(config.rule["never"].categories, ["other"]);
	assert_eq!(config.permit.len(), 1);
	assert_eq!(config.permit["example"].length.default, Some(Duration::from_secs(30 * 60)));
	assert_eq!(config.permit["example"].length.maximum, Some(Duration::from_secs(40 * 60)));
	assert_eq!(config.permit["example"].cooldown, Some(Duration::from_secs(20 * 60 * 60)));
	assert_eq!(
		config.permit["example"].available,
		Some(TimeRange { since: NaiveTime::from_hms(20, 0, 0), until: NaiveTime::from_hms(0, 0, 0) })
	);
	assert_eq!(config.permit["example"].categories, ["other"]);
}

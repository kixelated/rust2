//! Runs a script of executor steps and returns the log: the native side of the
//! differential test, which `tests/task.ts` mirrors step for step on the translation.
//!
//! Each step is two bytes, an op and its argument. Channels and tasks are picked by the
//! argument, modulo how many there are; a step with nothing to pick does nothing. Once
//! the script ends, every task drops in order, then every channel.

use crate::exec::Executor;
use crate::log::Log;
use crate::model::{self, Channel, Guard};

/// Run one script.
pub fn run(input: &[u8]) -> Vec<u8> {
	let log = Log::new();
	let mut exec = Executor::new(log.clone());
	let mut channels: Vec<Channel> = Vec::new();
	for step in input.chunks(2) {
		let (op, arg) = (step[0], step.get(1).copied().unwrap_or(0));
		let pick = |len: usize| (len > 0).then(|| usize::from(arg) % len);
		match op % 11 {
			0 => {
				log.note(format!("c{} open", channels.len()));
				channels.push(Channel::open());
			}
			1 => {
				if let Some(c) = pick(channels.len()) {
					let guard = Guard::new(exec.len() as u32, log.clone());
					exec.spawn(model::sum(channels[c].consumer(), guard));
				}
			}
			2 => {
				if let Some(c) = pick(channels.len()) {
					let depth = [1, 4, 16][usize::from(arg / 16) % 3];
					exec.spawn(model::drain(channels[c].consumer(), depth));
				}
			}
			3 => {
				if let Some(c) = pick(channels.len()) {
					log.note(format!("c{c} send {arg}"));
					if !channels[c].send(u64::from(arg)) {
						log.note(format!("c{c} closed"));
					}
				}
			}
			4 => {
				if let Some(c) = pick(channels.len()) {
					log.note(format!("c{c} close"));
					channels[c].close();
				}
			}
			5 => {
				if let Some(c) = pick(channels.len()) {
					log.note(format!("c{c} clone"));
					channels[c].clone_producer();
				}
			}
			6 => exec.run(),
			7 => _ = exec.step(),
			8 => {
				if let Some(t) = pick(exec.len()) {
					exec.poll(t);
				}
			}
			9 => {
				if let Some(t) = pick(exec.len()) {
					exec.drop(t);
				}
			}
			_ => {
				if let Some(c) = pick(channels.len()) {
					let guard = Guard::new(exec.len() as u32, log.clone());
					exec.spawn(model::blocks(channels[c].consumer(), guard, arg >= 128));
				}
			}
		}
	}
	for t in 0..exec.len() {
		exec.drop(t);
	}
	for (c, channel) in channels.into_iter().enumerate() {
		log.note(format!("c{c} drop"));
		drop(channel);
	}
	log.text()
}

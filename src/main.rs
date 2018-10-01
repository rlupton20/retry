/// Retry is a command line utility to help run commands until success
#[macro_use]
extern crate quicli;
#[macro_use]
extern crate failure;

use quicli::prelude::*;
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime};

/// Retry runs commands in a loop until they succeed
#[derive(Debug, StructOpt)]
struct RetryCli {
    /// The command which you would like to run and retry
    #[structopt(raw(required = "true", min_values = "1"))]
    command: Vec<String>,
    #[structopt(flatten)]
    verbosity: Verbosity,
    #[structopt(long = "timeout", short = "t")]
    /// Timeout (in seconds)
    timeout: Option<f64>,
    #[structopt(long = "interval", short = "i")]
    /// Interval between attempts (in seconds)
    interval: Option<f64>,
    #[structopt(long = "maximum-iterations", short = "m")]
    maximum_iterations: Option<usize>,
}

/// Errors for retry
#[derive(Debug, Fail)]
enum RetryError {
    #[fail(display = "Retrying command did not succeed due to timeout")]
    Timeout(),
    #[fail(display = "Retrying command reached maximum iterations")]
    MaximumIterations(),
}

#[derive(Debug)]
struct LoopManager {
    start_of_day: SystemTime,
    timeout: Option<f64>,
    interval: Option<f64>,
    maximum_iterations: Option<usize>,
    iteration: usize,
}

impl RetryCli {
    fn build_loop_manager(&self) -> LoopManager {
        LoopManager {
            start_of_day: SystemTime::now(),
            timeout: self.timeout,
            interval: self.interval,
            maximum_iterations: self.maximum_iterations,
            iteration: 0,
        }
    }
}

fn milliseconds(time_s: f64) -> u64 {
    (time_s * 1000.0) as u64
}

impl LoopManager {
    fn interval(&self) -> Result<Duration> {
        if let Some(i) = self.interval {
            Ok(
                Duration::from_millis(milliseconds(i * (self.iteration as f64)))
                    - self.start_of_day.elapsed()?,
            )
        } else {
            Ok(Duration::from_secs(0))
        }
    }

    fn elapsed(&self) -> Result<Duration> {
        Ok(self.start_of_day.elapsed()?)
    }

    fn step(&mut self) -> Result<()> {
        if let Some(t) = self.timeout {
            if !(self.elapsed()? < Duration::from_millis(milliseconds(t))) {
                return Err(RetryError::Timeout())?;
            }
        }

        if let Some(m) = self.maximum_iterations {
            if !(self.iteration + 1 < m) {
                return Err(RetryError::MaximumIterations())?;
            }
        }

        self.iteration += 1;
        Ok(())
    }

    fn status(&self) -> Result<String> {
        Ok(format!(
            "Elapsed time: {:?}; Iteration: {}",
            self.elapsed()?,
            self.iteration
        ))
    }
}

main!(|args: RetryCli, log_level: verbosity| {
    debug!("Got arguments: {:?}", args);

    let (cmd, cmd_args) = args.command.split_at(1);

    let mut loop_manager = args.build_loop_manager();
    debug!("Loop manager initialized: {:?}", loop_manager);

    loop {
        let status = Command::new(&cmd[0]).args(cmd_args).status()?;
        if let Some(rc) = status.code() {
            if rc == 0 {
                break;
            }
        }

        loop_manager.step()?;

        debug!("Loop manager status: {:?}", loop_manager.status()?);

        thread::sleep(loop_manager.interval()?);
    }
});

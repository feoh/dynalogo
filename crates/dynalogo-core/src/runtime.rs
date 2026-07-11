//! Simulation/VM runtime thread.
//!
//! The REPL or UI sends commands to the runtime; the VM and simulation tick
//! live on the runtime thread and emit output/snapshot events. This preserves
//! the Atari LOGO feel: user input does not block moving turtles.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::sim::{FixedTimestep, SimConfig, TurtleSnapshot};
use crate::vm::Vm;

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeCommand {
    Eval(String),
    Shutdown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeEvent {
    Output(String),
    Error(String),
    Snapshot(TurtleSnapshot),
    Stopped,
}

#[derive(Debug)]
pub struct RuntimeHandle {
    commands: Sender<RuntimeCommand>,
    events: Receiver<RuntimeEvent>,
    join: Option<JoinHandle<()>>,
}

impl RuntimeHandle {
    pub fn spawn(config: SimConfig) -> Self {
        let (commands_tx, commands_rx) = mpsc::channel();
        let (events_tx, events_rx) = mpsc::channel();
        let join = thread::spawn(move || runtime_loop(config, commands_rx, events_tx));
        Self {
            commands: commands_tx,
            events: events_rx,
            join: Some(join),
        }
    }

    pub fn send(&self, command: RuntimeCommand) -> Result<(), mpsc::SendError<RuntimeCommand>> {
        self.commands.send(command)
    }

    pub fn recv_event(&self, timeout: Duration) -> Result<RuntimeEvent, RecvTimeoutError> {
        self.events.recv_timeout(timeout)
    }

    pub fn shutdown(mut self) -> thread::Result<()> {
        let _ = self.commands.send(RuntimeCommand::Shutdown);
        if let Some(join) = self.join.take() {
            join.join()
        } else {
            Ok(())
        }
    }
}

impl Drop for RuntimeHandle {
    fn drop(&mut self) {
        let _ = self.commands.send(RuntimeCommand::Shutdown);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn runtime_loop(
    config: SimConfig,
    commands: Receiver<RuntimeCommand>,
    events: Sender<RuntimeEvent>,
) {
    let mut vm = Vm::new();
    let mut timestep = FixedTimestep::new(config);

    loop {
        match commands.recv_timeout(config.tick) {
            Ok(RuntimeCommand::Eval(source)) => match vm.eval_source(&source) {
                Ok(result) => {
                    if !result.output.is_empty() {
                        let _ = events.send(RuntimeEvent::Output(result.output));
                    }
                    for value in result.stack {
                        let _ = events.send(RuntimeEvent::Output(format!(
                            "{}\n",
                            value.show(vm.interner())
                        )));
                    }
                }
                Err(error) => {
                    let _ = events.send(RuntimeEvent::Error(error.to_string()));
                }
            },
            Ok(RuntimeCommand::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }

        let tick_seconds = config.tick.as_secs_f64();
        timestep.advance(config.tick, |tick| {
            if let Err(error) = vm.dynaturtle_tick(tick_seconds) {
                let _ = events.send(RuntimeEvent::Error(error.to_string()));
            }
            let snapshot = TurtleSnapshot::single(tick, vm.turtle().state());
            let _ = events.send(RuntimeEvent::Snapshot(snapshot));
        });
    }

    let _ = events.send(RuntimeEvent::Stopped);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_config() -> SimConfig {
        SimConfig {
            tick: Duration::from_millis(1),
            max_steps_per_frame: 4,
        }
    }

    #[test]
    fn runtime_evaluates_commands_on_worker_thread() {
        let runtime = RuntimeHandle::spawn(tiny_config());
        runtime
            .send(RuntimeCommand::Eval("print sum 2 3".to_string()))
            .unwrap();

        let mut saw_output = false;
        for _ in 0..20 {
            if let Ok(RuntimeEvent::Output(output)) = runtime.recv_event(Duration::from_millis(50))
            {
                saw_output = output == "5\n";
                if saw_output {
                    break;
                }
            }
        }
        assert!(saw_output);
        runtime.shutdown().unwrap();
    }

    #[test]
    fn runtime_emits_snapshots_without_commands() {
        let runtime = RuntimeHandle::spawn(tiny_config());
        let mut saw_snapshot = false;
        for _ in 0..20 {
            if matches!(
                runtime.recv_event(Duration::from_millis(50)),
                Ok(RuntimeEvent::Snapshot(_))
            ) {
                saw_snapshot = true;
                break;
            }
        }
        assert!(saw_snapshot);
        runtime.shutdown().unwrap();
    }

    #[test]
    fn runtime_reports_eval_errors() {
        let runtime = RuntimeHandle::spawn(tiny_config());
        runtime
            .send(RuntimeCommand::Eval("print :missing".to_string()))
            .unwrap();
        let mut saw_error = false;
        for _ in 0..20 {
            if let Ok(RuntimeEvent::Error(error)) = runtime.recv_event(Duration::from_millis(50)) {
                saw_error = error.contains("missing has no value");
                if saw_error {
                    break;
                }
            }
        }
        assert!(saw_error);
        runtime.shutdown().unwrap();
    }
}

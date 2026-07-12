//! Simulation runtime utilities.
//!
//! DynaLOGO supports two execution styles:
//!
//! - a native threaded runtime (`RuntimeHandle`) for desktop frontends
//! - a cooperative runtime (`CooperativeRuntime`) that can be driven from a
//!   browser event loop such as `requestAnimationFrame`
//!
//! The cooperative runtime is the important WASM-friendly abstraction: callers
//! feed it elapsed frame time and poll the resulting output/snapshot events.

use std::collections::VecDeque;
use std::time::Duration;

use crate::sim::{FixedTimestep, SimConfig, TurtleSnapshot};
use crate::vm::{RunResult, Vm};

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
pub struct CooperativeRuntime {
    vm: Vm,
    timestep: FixedTimestep,
    events: VecDeque<RuntimeEvent>,
}

impl CooperativeRuntime {
    pub fn new(config: SimConfig) -> Self {
        Self {
            vm: Vm::new(),
            timestep: FixedTimestep::new(config),
            events: VecDeque::new(),
        }
    }

    pub fn vm(&self) -> &Vm {
        &self.vm
    }

    pub fn vm_mut(&mut self) -> &mut Vm {
        &mut self.vm
    }

    pub fn eval(&mut self, source: impl AsRef<str>) {
        match self.vm.eval_source(source.as_ref()) {
            Ok(result) => {
                emit_eval_result(&mut self.vm, result, |event| self.events.push_back(event))
            }
            Err(error) => self
                .events
                .push_back(RuntimeEvent::Error(error.to_string())),
        }
    }

    /// Advances the VM/simulation cooperatively by `elapsed` wall-clock time.
    ///
    /// Browser frontends should call this from `requestAnimationFrame`, passing
    /// the frame delta. Every completed fixed tick emits a `Snapshot` event.
    pub fn advance(&mut self, elapsed: Duration) -> usize {
        let tick_seconds = self.timestep.config().tick.as_secs_f64();
        self.timestep.advance(elapsed, |tick| {
            if let Err(error) = self.vm.dynaturtle_tick(tick_seconds) {
                self.events
                    .push_back(RuntimeEvent::Error(error.to_string()));
            }
            self.events
                .push_back(RuntimeEvent::Snapshot(TurtleSnapshot {
                    tick,
                    turtles: self.vm.turtles().snapshots(),
                }));
        })
    }

    pub fn pop_event(&mut self) -> Option<RuntimeEvent> {
        self.events.pop_front()
    }

    pub fn drain_events(&mut self) -> Vec<RuntimeEvent> {
        self.events.drain(..).collect()
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod threaded {
    use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use crate::runtime::{emit_eval_result, RuntimeCommand, RuntimeEvent};
    use crate::sim::{FixedTimestep, SimConfig, TurtleSnapshot};
    use crate::vm::Vm;

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
                    Ok(result) => emit_eval_result(&mut vm, result, |event| {
                        let _ = events.send(event);
                    }),
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
                let _ = events.send(RuntimeEvent::Snapshot(TurtleSnapshot {
                    tick,
                    turtles: vm.turtles().snapshots(),
                }));
            });
        }

        let _ = events.send(RuntimeEvent::Stopped);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use threaded::RuntimeHandle;

fn emit_eval_result(vm: &mut Vm, result: RunResult, mut emit: impl FnMut(RuntimeEvent)) {
    if !result.output.is_empty() {
        emit(RuntimeEvent::Output(result.output));
    }
    for value in result.stack {
        emit(RuntimeEvent::Output(format!(
            "{}\n",
            value.show(vm.interner())
        )));
    }
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
    fn cooperative_runtime_evaluates_commands_without_threads() {
        let mut runtime = CooperativeRuntime::new(tiny_config());
        runtime.eval("print sum 2 3");
        assert_eq!(
            runtime.pop_event(),
            Some(RuntimeEvent::Output("5\n".to_string()))
        );
    }

    #[test]
    fn cooperative_runtime_emits_snapshots_when_advanced() {
        let mut runtime = CooperativeRuntime::new(tiny_config());
        let steps = runtime.advance(Duration::from_millis(5));
        assert!(steps > 0);
        assert!(runtime
            .drain_events()
            .into_iter()
            .any(|event| matches!(event, RuntimeEvent::Snapshot(_))));
    }

    #[test]
    fn cooperative_runtime_reports_eval_errors() {
        let mut runtime = CooperativeRuntime::new(tiny_config());
        runtime.eval("print :missing");
        let event = runtime.pop_event().expect("expected error event");
        match event {
            RuntimeEvent::Error(message) => assert!(message.contains("missing has no value")),
            other => panic!("expected error event, got {other:?}"),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
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

    #[cfg(not(target_arch = "wasm32"))]
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

    #[cfg(not(target_arch = "wasm32"))]
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

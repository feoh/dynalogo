use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use dynalogo_core::vm::{ControlFlow, Vm};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("--eval") | Some("-e") => {
            let source = args.collect::<Vec<_>>().join(" ");
            if source.is_empty() {
                return Err("--eval requires Logo source".to_string());
            }
            let mut vm = Vm::new();
            eval_and_print(&mut vm, &source)
        }
        Some("--help") | Some("-h") => {
            print_help();
            Ok(())
        }
        Some(arg) => Err(format!("unknown argument: {arg}\nTry --help.")),
        None => repl(),
    }
}

fn print_help() {
    println!(
        "DynaLOGO\n\nUSAGE:\n    dynalogo             Start the terminal REPL\n    dynalogo --eval SRC  Evaluate one Logo source string\n\nREPL:\n    ? prompt: enter Logo instructions\n    > prompt: continuing a TO ... END procedure definition\n    bye / exit / quit: leave the REPL"
    );
}

fn repl() -> Result<(), String> {
    let stdin = io::stdin();
    let mut vm = Vm::new();
    let mut definition_buffer: Option<String> = None;

    loop {
        let in_definition = definition_buffer.is_some();
        print!("{} ", if in_definition { ">" } else { "?" });
        io::stdout().flush().map_err(|error| error.to_string())?;

        let mut line = String::new();
        let bytes = stdin
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        if bytes == 0 {
            println!();
            break;
        }

        let trimmed = line.trim();
        if definition_buffer.is_none()
            && matches!(
                trimmed.to_ascii_lowercase().as_str(),
                "bye" | "exit" | "quit"
            )
        {
            break;
        }
        if trimmed.is_empty() {
            continue;
        }

        if let Some(buffer) = &mut definition_buffer {
            buffer.push_str(&line);
            if trimmed.eq_ignore_ascii_case("end") {
                let source = definition_buffer.take().expect("definition buffer exists");
                eval_and_print(&mut vm, &source)?;
            }
            continue;
        }

        if starts_with_logo_word(trimmed, "to") || starts_with_logo_word(trimmed, ".macro") {
            definition_buffer = Some(line);
            continue;
        }

        eval_and_print(&mut vm, &line)?;
    }

    Ok(())
}

fn eval_and_print(vm: &mut Vm, source: &str) -> Result<(), String> {
    let result = vm.eval_source(source).map_err(|error| error.to_string())?;
    print!("{}", result.output);
    vm.clear_output();
    io::stdout().flush().map_err(|error| error.to_string())?;

    match result.control {
        ControlFlow::None => {
            for value in result.stack {
                println!("{}", value.show(vm.interner()));
            }
        }
        ControlFlow::Output(value) => {
            // At top level, show the value rather than dropping it silently.
            println!("{}", value.show(vm.interner()));
        }
        ControlFlow::Stop => {}
        ControlFlow::Continue => {
            return Err("CONTINUE can only be used inside PAUSE".to_string());
        }
        ControlFlow::Throw { tag, value } => {
            println!(
                "Uncaught THROW {} {}",
                tag.show(vm.interner()),
                value.show(vm.interner())
            );
        }
    }
    Ok(())
}

fn starts_with_logo_word(line: &str, word: &str) -> bool {
    let mut parts = line.split_whitespace();
    matches!(parts.next(), Some(first) if first.eq_ignore_ascii_case(word))
}

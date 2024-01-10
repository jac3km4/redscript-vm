use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use redscript::bundle::{ConstantPool, ScriptBundle};
use redscript_compiler::error::Error;
use redscript_compiler::source_map::{Files, SourceFilter};
use redscript_compiler::unit::CompilationUnit;
use redscript_vm::{args, native, VM};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use serde::Deserialize;

mod test;

const HISTORY_FILE: &str = "redscript-history.txt";

fn main() -> anyhow::Result<()> {
    let location = std::env::current_dir()?.join("redscript.toml");
    match ShellConfig::load(&location) {
        Ok(config) => {
            let mut file = io::BufReader::new(File::open(&config.bundle_path)?);
            let bundle = ScriptBundle::load(&mut file)?;
            repl(bundle.pool, &config)
        }
        Err(error) => {
            println!("Failed to load the shell config (redscript.toml is required)");
            Err(error.into())
        }
    }
}

fn repl(pool: ConstantPool, config: &ShellConfig) -> anyhow::Result<()> {
    println!("Welcome to the redscript shell! Type 'help' for more information.");

    let mut rl = DefaultEditor::new()?;
    if rl.load_history(HISTORY_FILE).is_err() {
        println!("No previous history");
    }
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;
                match Command::parse(&line) {
                    Ok(cmd) => match execute(cmd, pool.clone(), config) {
                        Ok(true) => break,
                        Ok(false) => {}
                        Err(err) => println!("{:?}", err),
                    },
                    Err(err) => println!("{}", err),
                }
            }
            Err(ReadlineError::Interrupted) => {
                break;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                println!("Error: {}", err);
                break;
            }
        }
    }
    rl.save_history(HISTORY_FILE)?;
    Ok(())
}

fn execute(command: Command<'_>, pool: ConstantPool, config: &ShellConfig) -> anyhow::Result<bool> {
    match command {
        Command::RunMain => {
            run_function(pool, "main;", config)?;
            Ok(false)
        }
        Command::Run(func) => {
            run_function(pool, func, config)?;
            Ok(false)
        }
        Command::Test(suite) => {
            test::run_suite(pool, suite, config)?;
            Ok(false)
        }
        Command::Help => {
            println!("Available commands: runMain, run [function], test [suite], help, exit");
            Ok(false)
        }
        Command::Exit => Ok(true),
    }
}

fn run_function(mut pool: ConstantPool, func_name: &str, config: &ShellConfig) -> anyhow::Result<()> {
    let sources = Files::from_dir(&config.source_dir, &SourceFilter::None)?;
    CompilationUnit::new_with_defaults(&mut pool)?.compile_files(&sources)?;

    let mut vm = VM::new(&pool);
    native::register_natives(&mut vm, |str| println!("{}", str));

    let main = vm
        .metadata()
        .get_function(func_name)
        .ok_or_else(|| anyhow::anyhow!("no main function"))?;
    let out = vm.call_with_callback(main, args!(), |res| res.map(|val| val.to_string(&pool)))?;
    if let Some(res) = out {
        println!("result: {}", res);
    }
    Ok(())
}

enum Command<'inp> {
    RunMain,
    Run(&'inp str),
    Test(&'inp str),
    Help,
    Exit,
}

impl<'inp> Command<'inp> {
    fn parse(input: &'inp str) -> Result<Self, &'static str> {
        let parts = input.split(' ').collect::<Vec<_>>();
        match parts.as_slice() {
            ["runMain"] => Ok(Command::RunMain),
            ["run", method] => Ok(Command::Run(method)),
            ["test", suite] => Ok(Command::Test(suite)),
            ["help"] => Ok(Command::Help),
            ["exit"] => Ok(Command::Exit),
            _ => Err("Invalid command, enter 'help' for more information"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ShellConfig {
    bundle_path: PathBuf,
    #[serde(default = "ShellConfig::default_source_dir")]
    source_dir: PathBuf,
    #[serde(default = "ShellConfig::default_test_dir")]
    test_dir: PathBuf,
}

impl ShellConfig {
    pub fn load(path: &Path) -> Result<Self, Error> {
        let contents = std::fs::read_to_string(path)?;
        let res =
            toml::from_str(&contents).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
        Ok(res)
    }

    fn default_source_dir() -> PathBuf {
        "src".into()
    }

    fn default_test_dir() -> PathBuf {
        "test".into()
    }
}

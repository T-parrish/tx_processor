use bank::domain::Transaction;
use bank::domain::{History, Account};
use bank::engine::{Machine, Task};
use log::error;
use std::collections::HashMap;
use std::fs::File;

use std::env::args;
use std::io::Write;
use std::sync::mpsc::channel;
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = args().collect();
    let mut history = History::new();
    let mut accounts = HashMap::<u16, Account>::new();

    let (tx, rx) = channel();
    let handle = thread::spawn(move || {
        let tx_file = &args[1];
        let file = File::open(tx_file).expect("Failed to open file");
        let mut reader = csv::Reader::from_reader(file);
        for record in reader.deserialize::<Transaction>() {
            match record {
                Ok(out) => tx.send(out).expect("Failed to send record"),
                Err(e) => error!("Failed to deserialize record: {e}"),
            };
        }
    });

    while let Ok(record) = rx.recv() {
        let mut task = Task::new(&mut history, &mut accounts, record);
        let res = &mut task.run();
        match res {
            Ok(_) => (),
            Err(e) => error!("{}", e),
        };
    }

    handle.join().expect("Failed to join thread handle");

    let mut writer = csv::Writer::from_writer(vec![]);

    for act in accounts.values() {
        writer.serialize(act)?
    }

    // let output = String::from_utf8(writer.into_inner()?)?;
    let inner = writer.into_inner()?;
    std::io::stdout().write_all(&inner)?;

    Ok(())
}

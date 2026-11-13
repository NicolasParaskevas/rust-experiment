use std::{error::Error, fs::File, io, process::ExitCode};

use rust_experiment::services::{
    csv_streamer::CsvStreamer,
    payment_engine::PaymentEngine,
};

fn main() -> ExitCode {
    if run().is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let path = args.next().ok_or("Usage: rust-experiment <input.csv>")?;

    if args.next().is_some() {
        return Err("Usage: rust-experiment <input.csv>".into());
    }

    let mut engine = PaymentEngine::new();

    for transaction in CsvStreamer::new(File::open(path)?)? {
        match transaction {
            Ok(transaction) => engine.process(transaction),
            Err(error) if error.kind() == io::ErrorKind::InvalidData => continue,
            Err(error) => return Err(error.into()),
        }
    }

    let stdout = io::stdout();
    let mut output = csv::Writer::from_writer(stdout.lock());

    output.write_record(["client", "available", "held", "total", "locked"])?;

    for account in engine.accounts() {
        let Some(total) = account.total() else {
            continue;
        };

        output.write_record([
            account.id.to_string(),
            account.available.to_string(),
            account.held.to_string(),
            total.to_string(),
            account.locked.to_string(),
        ])?;
    }

    output.flush()?;
    Ok(())
}
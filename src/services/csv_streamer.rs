use std::io::{self, Read};

use csv::{Reader, ReaderBuilder, StringRecord, Trim};
use rust_decimal::Decimal;

use crate::models::transaction::{Transaction, TransactionType};

pub struct CsvStreamer<R: Read> {
    reader: Reader<R>,
    record: StringRecord,
}

impl<R: Read> CsvStreamer<R> {
    pub fn new(input: R) -> io::Result<Self> {
        let mut reader = ReaderBuilder::new()
            .trim(Trim::All)
            .flexible(true)
            .from_reader(input);
        let headers = reader.headers().map_err(csv_error)?;
        if headers.iter().ne(["type", "client", "tx", "amount"]) {
            return Err(invalid_data(
                "line 1: expected header type,client,tx,amount",
            ));
        }
        Ok(Self {
            reader,
            record: StringRecord::new(),
        })
    }
}

impl<R: Read> Iterator for CsvStreamer<R> {
    type Item = io::Result<Transaction>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_record(&mut self.record) {
            Ok(true) => {
                let line = self.record.position().map_or(1, |position| position.line());
                Some(
                    parse_transaction(&self.record)
                        .map_err(|error| invalid_data(format!("line {line}: {error}"))),
                )
            }
            Ok(false) => None,
            Err(error) => Some(Err(csv_error(error))),
        }
    }
}

fn parse_transaction(record: &StringRecord) -> io::Result<Transaction> {
    if !(3..=4).contains(&record.len()) {
        return Err(invalid_data("expected 3 or 4 fields"));
    }
    let transaction_type = match field(record, 0, "type")? {
        "deposit" => TransactionType::Deposit,
        "withdrawal" => TransactionType::Withdrawal,
        "dispute" => TransactionType::Dispute,
        "resolve" => TransactionType::Resolve,
        "chargeback" => TransactionType::Chargeback,
        value => return Err(invalid_data(format!("unknown transaction type: {value}"))),
    };
    let client = field(record, 1, "client")?
        .parse()
        .map_err(|error| invalid_data(format!("invalid client: {error}")))?;
    let tx = field(record, 2, "tx")?
        .parse()
        .map_err(|error| invalid_data(format!("invalid tx: {error}")))?;

    let amount_text = record.get(3).unwrap_or("");
    let amount = if amount_text.is_empty() {
        match transaction_type {
            TransactionType::Deposit | TransactionType::Withdrawal => {
                return Err(invalid_data("missing amount"));
            }
            _ => Decimal::ZERO,
        }
    } else {
        let amount: Decimal = amount_text
            .parse()
            .map_err(|error| invalid_data(format!("invalid amount: {error}")))?;
        if amount.scale() > 4 {
            return Err(invalid_data("amount must have at most four decimal places"));
        }
        amount
    };
    Ok(Transaction {
        transaction_type,
        client,
        tx,
        amount,
    })
}

fn field<'a>(record: &'a StringRecord, index: usize, name: &str) -> io::Result<&'a str> {
    record
        .get(index)
        .ok_or_else(|| invalid_data(format!("missing {name}")))
}

fn csv_error(error: csv::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

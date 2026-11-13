use std::io;

use rust_decimal::Decimal;
use rust_experiment::{models::transaction::TransactionType, services::csv_streamer::CsvStreamer};

#[test]
fn accepts_whitespace_quoted_fields_and_maximum_ids() {
    let input = "type, client, tx, amount\n\"deposit\", 65535, 4294967295, 1.0001\n";
    let transaction = CsvStreamer::new(input.as_bytes())
        .unwrap()
        .next()
        .unwrap()
        .unwrap();
    assert_eq!(transaction.transaction_type, TransactionType::Deposit);
    assert_eq!(transaction.client, u16::MAX);
    assert_eq!(transaction.tx, u32::MAX);
    assert_eq!(transaction.amount, Decimal::new(10001, 4));
}

#[test]
fn accepts_missing_or_blank_amounts_for_dispute_events() {
    for kind in ["dispute", "resolve", "chargeback"] {
        for suffix in ["", ",", ",   "] {
            let input = format!("type,client,tx,amount\n{kind},1,2{suffix}\n");
            assert!(
                CsvStreamer::new(input.as_bytes())
                    .unwrap()
                    .next()
                    .unwrap()
                    .is_ok()
            );
        }
    }
}

#[test]
fn payments_require_amounts() {
    for kind in ["deposit", "withdrawal"] {
        for suffix in ["", ","] {
            let input = format!("type,client,tx,amount\n{kind},1,2{suffix}\n");
            let error = CsvStreamer::new(input.as_bytes())
                .unwrap()
                .next()
                .unwrap()
                .unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            assert!(error.to_string().contains("line 2: missing amount"));
        }
    }
}

#[test]
fn rejects_missing_incorrect_or_reordered_headers() {
    for header in [
        "",
        "deposit,1,1,10",
        "type,tx,client,amount",
        "type,client,tx",
        "oops,client,tx,amount",
    ] {
        let input = format!("{header}\ndeposit,1,2,10\n");
        assert!(CsvStreamer::new(input.as_bytes()).is_err(), "{header}");
    }
}

#[test]
fn header_only_input_has_no_transactions() {
    let mut reader = CsvStreamer::new("type,client,tx,amount\n".as_bytes()).unwrap();
    assert!(reader.next().is_none());
}

#[test]
fn rejects_extra_or_missing_fields() {
    for row in ["deposit,1", "deposit,1,2,10,extra"] {
        let input = format!("type,client,tx,amount\n{row}\n");
        let error = CsvStreamer::new(input.as_bytes())
            .unwrap()
            .next()
            .unwrap()
            .unwrap_err();
        assert!(error.to_string().contains("line 2: expected 3 or 4 fields"));
    }
}

#[test]
fn invalid_values_report_the_actual_line() {
    for (row, expected) in [
        ("other,1,2,10", "unknown transaction type"),
        ("deposit,65536,2,10", "invalid client"),
        ("deposit,1,4294967296,10", "invalid tx"),
        ("deposit,1,2,nope", "invalid amount"),
        ("deposit,1,2,1.00001", "four decimal places"),
    ] {
        let input = format!("type,client,tx,amount\ndeposit,1,1,10\n{row}\n");
        let mut reader = CsvStreamer::new(input.as_bytes()).unwrap();
        assert!(reader.next().unwrap().is_ok());
        let error = reader.next().unwrap().unwrap_err().to_string();
        assert!(error.contains("line 3:"), "{error}");
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn continues_after_a_rejected_row() {
    let input = b"type,client,tx,amount\ndeposit,1,1,nope\ndeposit,1,2,1.0001\n";
    let mut reader = CsvStreamer::new(&input[..]).unwrap();
    assert!(reader.next().unwrap().is_err());
    assert_eq!(reader.next().unwrap().unwrap().tx, 2);
    assert!(reader.next().is_none());
}

#[test]
fn invalid_utf8_is_a_skippable_record_error() {
    let input = b"type,client,tx,amount\ndeposit,1,1,\xff\ndeposit,1,2,5\n";
    let mut reader = CsvStreamer::new(&input[..]).unwrap();
    assert_eq!(
        reader.next().unwrap().unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(reader.next().unwrap().unwrap().amount, Decimal::from(5));
}

#[test]
fn accepts_bom_crlf_blank_lines_and_no_final_newline() {
    let input = "\u{feff}type,client,tx,amount\r\n\r\ndeposit,0,0,0.0001";
    let mut reader = CsvStreamer::new(input.as_bytes()).unwrap();
    let transaction = reader.next().unwrap().unwrap();
    assert_eq!(transaction.client, 0);
    assert_eq!(transaction.tx, 0);
    assert_eq!(transaction.amount, Decimal::new(1, 4));
    assert!(reader.next().is_none());
}

#[test]
fn rejects_empty_and_headerless_input() {
    assert!(CsvStreamer::new(&b""[..]).is_err());
    assert!(CsvStreamer::new(&b"deposit,1,1,5\n"[..]).is_err());
}

#[test]
fn exact_decimal_parser_does_not_round_invalid_precision_into_valid_input() {
    let input = b"type,client,tx,amount\ndeposit,1,1,7922816251426433759354395.03356\n";
    assert!(
        CsvStreamer::new(&input[..])
            .unwrap()
            .next()
            .unwrap()
            .is_err()
    );
}

#[test]
fn rejects_non_numbers_and_out_of_range_amounts() {
    for amount in ["NaN", "inf", "79228162514264337593543950336"] {
        let input = format!("type,client,tx,amount\ndeposit,1,1,{amount}\n");
        assert!(
            CsvStreamer::new(input.as_bytes())
                .unwrap()
                .next()
                .unwrap()
                .is_err()
        );
    }
}

#[test]
fn underlying_read_failure_is_not_reported_as_a_bad_row() {
    use std::io::{Cursor, Read};
    struct FailingRead;
    impl Read for FailingRead {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("simulated disk failure"))
        }
    }
    let input = Cursor::new(b"type,client,tx,amount\ndeposit,1,1,5\n").chain(FailingRead);
    let mut reader = CsvStreamer::new(input).unwrap();
    assert!(reader.next().unwrap().is_ok());
    assert_ne!(
        reader.next().unwrap().unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    assert!(reader.next().is_none());
}

#[test]
fn quoted_multiline_field_does_not_break_record_boundaries() {
    let input = b"type,client,tx,amount\n\"deposit\n\",1,1,10\ndeposit,1,2,20\n";
    let rows: Vec<_> = CsvStreamer::new(&input[..])
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].tx, 2);
}

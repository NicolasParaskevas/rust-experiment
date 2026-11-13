# Test cases and sample CSV files

```sh
cargo test
```

`payments.rs` tests the engine directly, independently of CSV parsing. It covers
deposits, exact withdrawals, insufficient available funds, held funds, negative
and zero amounts, duplicate deposits, all dispute transitions, repeated events,
re-disputing resolved deposits, cross-client and unknown references, successful
and failed withdrawal references, freezing, concurrent disputes on one account,
negative available/total balances, unordered and boundary IDs, and atomic
rejection of overflow or precision loss.

`csv_streamer.rs` covers headers, missing/extra fields, missing amounts, decimal
precision/range, continuation after invalid records, invalid UTF-8, whitespace,
quoted fields, multiline fields, BOM, CRLF, blank lines, no final newline,
line numbers, and classification of underlying read failures.

`cli.rs` compares full stdout against the expected CSVs, requires empty stderr,
checks exit statuses, and covers argument errors, missing files, invalid
headers, and output failure on Linux. Fixtures are small enough to inspect.

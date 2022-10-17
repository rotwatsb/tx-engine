## Purpose
This little program is a simple toy transaction processor. It is written in the spirit of learning, as a chance to play around with the serde and csv crates.

## Crates Used
1. [Serde](https://serde.rs/) for de/serialization
2. [CSV](https://docs.rs/csv/latest/csv/) for csv parsing
3. [Decimal](https://docs.rs/rust_decimal/latest/rust_decimal/) for calculation, rounding, and formatting of numbers

## Input
Input is expected to be a csv of transactions, in the same format as one of csvs included in this repo's test_data directory. Transactions can be any of (deposit, withdrawal, dispute, resolve, chargeback). The `deposit` and `withdrawal` transaction types have unique transaction ids. The others make reference to those transaction ids. They do not have unique transaction ids for themselves.

## Output
Output, written to stdout, is likewise in csv format. The ouput is the final balance of client accounts.

## Running

Run with `cargo run -- test_data/test1.csv`

## Testing

There are a few test csvs in the test_data folder that the code can run be against. And there are two small ruby scripts that generate larger csvs for testing.

There is a bloated unit test which makes various assertions after processing each of the five possible transaction types for a single client. Run it with `cargo test`

## Assumptions

1. Disputes can only be made on `deposit`-type transactions.
2. `dispute`, `resolve` and `chargeback` transactions are only valid if processed with the same client as that of the disputed `deposit` transaction.

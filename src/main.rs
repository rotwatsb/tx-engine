use csv::{ReaderBuilder, Trim};
use rust_decimal::Decimal;
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::io;
use std::process;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
enum TxAction {
    DEPOSIT,
    WITHDRAWAL,
    DISPUTE,
    RESOLVE,
    CHARGEBACK,
}

type TxAmount = Option<Decimal>;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Tx {
    #[serde(rename = "type")]
    action: TxAction,

    client: u16,
    tx: u32,

    #[serde(default)]
    amount: TxAmount,

    #[serde(default)]
    is_disputed: bool,
}

#[derive(Debug)]
struct Account {
    client: u16,
    available: Decimal,
    held: Decimal,
    is_locked: bool,
}

// use custom serialization here to both
// (1) avoid adding an otherwise unhelpful "locked" field to the Account struct
// (2) correctly serialize decimal values
impl Serialize for Account {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Account", 5)?;
        state.serialize_field("client", &self.client)?;
        state.serialize_field("available", &self.available.round_dp(4).normalize())?;
        state.serialize_field("held", &self.held.round_dp(4).normalize())?;
        state.serialize_field("total", &self.total().round_dp(4).normalize())?;
        state.serialize_field("locked", &self.is_locked)?;
        state.end()
    }
}

impl Account {
    fn total(&self) -> Decimal {
        self.available + self.held
    }

    fn deposit(&mut self, amount: TxAmount) {
        if !self.is_locked {
            if let Some(value) = amount {
                self.available += value;
            }
        }
    }

    fn withdraw(&mut self, amount: TxAmount) {
        if !self.is_locked {
            if let Some(value) = amount {
                if value <= self.available {
                    self.available -= value;
                }
            }
        }
    }

    fn hold(&mut self, amount: TxAmount) {
        if !self.is_locked {
            if let Some(value) = amount {
                self.available -= value;
                self.held += value;
            }
        }
    }

    fn release(&mut self, amount: TxAmount) {
        if !self.is_locked {
            if let Some(value) = amount {
                self.available += value;
                self.held -= value;
            }
        }
    }

    fn chargeback(&mut self, amount: TxAmount) {
        if !self.is_locked {
            if let Some(value) = amount {
                self.held -= value;
                self.is_locked = true;
            }
        }
    }
}

type AccountMap = HashMap<u16, Account>;
type TxMap = HashMap<u32, Tx>;

// support collect to map of transactions from an iterator over Tx
// ref:: https://doc.rust-lang.org/std/iter/trait.FromIterator.html
impl FromIterator<Tx> for TxMap {
    fn from_iter<I: IntoIterator<Item = Tx>>(iter: I) -> Self {
        iter.into_iter().map(|tx| (tx.tx, tx)).collect::<TxMap>()
    }
}

fn ensure_account(client: u16, accounts: &mut AccountMap) -> () {
    if !accounts.contains_key(&client) {
        accounts.insert(
            client,
            Account {
                client: client,
                available: Decimal::new(0, 0),
                held: Decimal::new(0, 0),
                is_locked: false,
            },
        );
    }
}

fn handle_dispute_action(account: &mut Account, disputed_tx: &mut Tx, action: TxAction) {
    match action {
        TxAction::DISPUTE => {
            // assumption: disallow disputes of transactions already under dispute
            if !disputed_tx.is_disputed {
                disputed_tx.is_disputed = true;
                account.hold(disputed_tx.amount);
            }
        }
        TxAction::RESOLVE => {
            // assumption: cannot resolve a transaction that isn't under dispute
            if disputed_tx.is_disputed {
                disputed_tx.is_disputed = false;
                account.release(disputed_tx.amount);
            }
        }
        TxAction::CHARGEBACK => {
            // assumption: cannot chargeback a transaction that isn't under dispute
            if disputed_tx.is_disputed {
                disputed_tx.is_disputed = false;
                account.chargeback(disputed_tx.amount);
            }
        }
        _ => (), // neither DEPOSIT nor WITHDRAWAL affect dispute lifecycle
    };
}

fn process_tx(
    tx: Tx,
    disputable_txs: &mut TxMap,
    accounts: &mut AccountMap,
) -> Result<(), Box<dyn Error>> {
    ensure_account(tx.client, accounts);

    if let Some(account) = accounts.get_mut(&tx.client) {
        match tx.action {
            TxAction::DEPOSIT => account.deposit(tx.amount),
            TxAction::WITHDRAWAL => account.withdraw(tx.amount),
            TxAction::DISPUTE | TxAction::RESOLVE | TxAction::CHARGEBACK => {
                if let Some(disputed_tx) = disputable_txs.get_mut(&tx.tx) {
                    // assumption: only allow disputes on deposits
                    // assumption: disallow client x to dispute tx of client y, where x != y
                    if disputed_tx.action == TxAction::DEPOSIT && disputed_tx.client == tx.client {
                        handle_dispute_action(account, disputed_tx, tx.action);
                    }
                }
            }
        };
    }

    Ok(())
}

fn balance_accounts(
    ordered_txs: Vec<Tx>,
    disputable_txs: &mut TxMap,
) -> Result<AccountMap, Box<dyn Error>> {
    let mut accounts: AccountMap = AccountMap::new();

    for tx in ordered_txs.into_iter() {
        process_tx(tx, disputable_txs, &mut accounts)?;
    }

    Ok(accounts)
}

fn write_accounts(accounts: AccountMap) -> Result<(), Box<dyn Error>> {
    let mut wtr = csv::Writer::from_writer(io::stdout());

    for account in accounts.into_values() {
        wtr.serialize(account)?;
    }

    wtr.flush()?;
    Ok(())
}

fn read_txs(path: &str) -> Result<Vec<Tx>, csv::Error> {
    // use flexible reader to accept csv rows like "dispute,1,2" and "dispute,1,2,"
    let mut reader = ReaderBuilder::new()
        .flexible(true)
        .trim(Trim::All)
        .from_path(path)?;
    reader.deserialize::<Tx>().collect()
}

fn safe_run(input_arg: &str) -> Result<(), Box<dyn Error>> {
    let ordered_txs = read_txs(input_arg)?;
    let mut disputable_txs: TxMap = ordered_txs
        .iter()
        .filter(|tx| tx.action == TxAction::DEPOSIT)
        .cloned()
        .collect();
    let accounts = balance_accounts(ordered_txs, &mut disputable_txs)?;
    write_accounts(accounts)?;

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let input_arg = &args[1];

    match safe_run(input_arg) {
        Ok(()) => (),
        Err(e) => {
            dbg!(e);
            process::exit(1);
        }
    }
}

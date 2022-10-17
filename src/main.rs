use csv::{ReaderBuilder, Trim, Reader};
use rust_decimal::Decimal;
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::io;
use std::process;
use std::fs::File;

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

impl Tx {
    fn is_disputable(&self) -> bool {
        // assumption: only allow disputes on deposits
        self.action == TxAction::DEPOSIT
    }
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

fn handle_dispute_action(account: &mut Account, disputed_tx: &mut Tx, action: &TxAction) {
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
    tx: &mut Tx,
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

                    // assumption: disallow client x to dispute tx of client y, where x != y
                    if disputed_tx.is_disputable() && disputed_tx.client == tx.client {
                        handle_dispute_action(account, disputed_tx, &tx.action);
                    }
                }
            }
        };
    }

    Ok(())
}

fn balance_accounts(
    mut tx_reader: Reader<File>
) -> Result<AccountMap, Box<dyn Error>> {
    let mut accounts: AccountMap = AccountMap::new();
    let mut disputable_txs = TxMap::new();

    let tx_iter = tx_reader.deserialize::<Tx>();

    for tx_result in tx_iter {
        let mut tx = tx_result?;
        process_tx(&mut tx, &mut disputable_txs, &mut accounts)?;

        if tx.is_disputable() {
            disputable_txs.insert(tx.tx, tx);
        }
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

fn safe_run(input_arg: &str) -> Result<(), Box<dyn Error>> {
    let tx_reader = ReaderBuilder::new()
        .flexible(true)
        .trim(Trim::All)
        .from_path(input_arg)?;

    let accounts = balance_accounts(tx_reader)?;
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

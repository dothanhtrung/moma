extern crate gtk;
extern crate rusqlite;
extern crate time;

use gtk::prelude::*;
use gtk::{Box, Builder, ComboBoxText, Dialog, Entry, Label, ListBox, SpinButton, TextView};
use rusqlite::Connection;
use time::Timespec;

pub fn db_connect() -> Connection {
    let conn = Connection::open("/tmp/test.db").unwrap();
    conn
}

pub fn db_init() {
    let conn = db_connect();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS wallets (
                  id            INTEGER PRIMARY KEY,
                  name          TEXT NOT NULL,
                  value         TEXT NOT NULL,
                  currency      TEXT NOT NULL,
                  isdefault     INTEGER, 
                  createdtime   TEXT NOT NULL
                  )",
        &[],
    ).unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS categories (
                  id            INTEGER PRIMARY KEY,
                  name          TEXT NOT NULL,
                  createdtime   TEXT NOT NULL
                  )",
        &[],
    ).unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS transactions (
                  id            INTEGER PRIMARY KEY,
                  name          TEXT NOT NULL,
                  description   TEXT,
                  value         TEXT NOT NULL,
                  transtime     TEXT NOT NULL,
                  createdtime   TEXT NOT NULL,
                  wallet        INTEGER,
                  category      INTEGER,
                  is_income     INTEGER,
                  FOREIGN KEY(wallet) REFERENCES wallet(id) ON DELETE CASCADE,
                  FOREIGN KEY(category) REFERENCES categories(id)
                  )",
        &[],
    ).unwrap();
}

pub struct Wallet {
    pub id: i64,
    pub name: String,
    pub value: String,
    pub currency: String,
    pub isdefault: bool,
    pub createdtime: Timespec,
}

pub fn get_all_wallets() -> Vec<Wallet> {
    let conn = db_connect();
    let mut wallets = Vec::new();
    let mut stmt = conn
        .prepare("SELECT id, name, value, currency, isdefault, createdtime FROM wallets")
        .unwrap();
    let wallets_querry = stmt
        .query_map(&[], |row| Wallet {
            id: row.get(0),
            name: row.get(1),
            value: row.get(2),
            currency: row.get(3),
            isdefault: row.get(4),
            createdtime: row.get(5),
        }).unwrap();

    for wallet in wallets_querry {
        wallets.push(wallet.unwrap());
    }

    wallets
}

pub struct Transaction {
    pub name: String,
    pub description: String,
    pub value: String,
    pub transtime: Timespec,
    pub createdtime: Timespec,
    pub wallet: i64,
    pub is_income: bool,
}

pub fn get_all_transactions(wid: i64) -> Vec<Transaction> {
    let conn = db_connect();
    let mut transactions = Vec::new();
    let mut stmt = conn
        .prepare(
            format!(
                "SELECT name, description, value, transtime, createdtime, is_income 
                FROM transactions WHERE wallet = \"{}\" ORDER BY transtime LIMIT 10",
                wid
            ).as_str(),
        ).unwrap();
    let transaction_querry = stmt
        .query_map(&[], |row| Transaction {
            name: row.get(0),
            description: row.get(1),
            value: row.get(2),
            transtime: row.get(3),
            createdtime: row.get(4),
            wallet: wid,
            is_income: row.get(5),
        }).unwrap();

    for transaction in transaction_querry {
        transactions.push(transaction.unwrap());
    }

    transactions
}

pub fn gtk_wallet_refresh(wid: i64, listbox: &ListBox, total: &Label, currency: &Label) {
    let mut transactions: Vec<Transaction> = Vec::new();
    if wid > 0 {
        let conn = db_connect();
        conn.query_row(
            "SELECT value, currency FROM wallets WHERE id = ?",
            &[&wid],
            |row| {
                let value: String = row.get(0);
                let _currency: String = row.get(1);
                total.set_text(&format!("{}", value).as_str());
                currency.set_text(&_currency);
            },
        ).unwrap();

        transactions = get_all_transactions(wid);

        for row in listbox.get_children() {
            listbox.remove(&row);
        }
    } else {
        total.set_text("");
        currency.set_text("");
    }

    for trans in transactions {
        listbox.prepend(&gtk_new_trans(trans.name, trans.value, trans.is_income));
    }
    listbox.show_all();
}

pub fn gtk_new_trans(_name: String, _value: String, is_income: bool) -> Box {
    let new_row = Box::new(gtk::Orientation::Horizontal, 0);
    let name = Label::new(_name.as_str());
    let value_str: String;
    if is_income {
        value_str = _value;
    } else {
        value_str = "-".to_owned() + _value.as_str();
    }
    let value = Label::new(value_str.as_str());
    new_row.pack_start(&name, true, false, 0);
    new_row.pack_start(&value, false, false, 0);

    new_row
}

pub fn db_new_trans(new_trans: &Transaction, is_income: bool) -> String {
    let conn = db_connect();
    conn.execute(
        "INSERT INTO transactions (name, description, value, transtime, createdtime, wallet, is_income)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        &[
            &new_trans.name,
            &new_trans.description,
            &new_trans.value,
            &new_trans.transtime,
            &new_trans.createdtime,
            &new_trans.wallet,
            &is_income,
        ],
    ).unwrap();

    let mut total_value: f64 = 0.0;
    let _ = conn.query_row(
        "SELECT value FROM wallets WHERE id = ?",
        &[&new_trans.wallet],
        |row| {
            let query_value: String = row.get(0);
            total_value = query_value.parse().unwrap();
            let trans_value: f64 = new_trans.value.parse().unwrap();

            if is_income {
                total_value += trans_value;
            } else {
                total_value -= trans_value;
            }

            conn.execute(
                "UPDATE wallets SET value = ?1 WHERE id = ?2",
                &[&total_value.to_string(), &new_trans.wallet],
            ).unwrap();
        },
    );

    total_value.to_string()
}

pub fn add_transaction(
    builder: &Builder,
    listbox: &ListBox,
    wallet_select: &ComboBoxText,
    is_income: bool,
) {
    let transaction_builder = Builder::new_from_string(include_str!("../res/transaction.glade"));
    let dialog: Dialog = transaction_builder.get_object("transaction").unwrap();
    let name: Entry = transaction_builder.get_object("name").unwrap();
    let description: TextView = transaction_builder.get_object("description").unwrap();
    let description_buffer = description.get_buffer().unwrap();
    let value: SpinButton = transaction_builder.get_object("value").unwrap();

    let ret = dialog.run();

    if ret == -5 {
        let new_trans = Transaction {
            name: name.get_text().unwrap(),
            description: description_buffer
                .get_text(
                    &description_buffer.get_start_iter(),
                    &description_buffer.get_end_iter(),
                    false,
                ).unwrap(),
            value: value.get_text().unwrap(),
            transtime: time::get_time(),
            createdtime: time::get_time(),
            wallet: wallet_select
                .get_active_id()
                .unwrap()
                .parse::<i64>()
                .unwrap(),
            is_income: is_income,
        };

        let _total: Label = builder.get_object("total").unwrap();
        _total.set_text(&db_new_trans(&new_trans, is_income));

        listbox.prepend(&gtk_new_trans(new_trans.name, new_trans.value, is_income));
        listbox.show_all();
    }

    dialog.destroy();
}

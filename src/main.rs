extern crate moma;
use moma::*;

extern crate gtk;
extern crate rusqlite;
extern crate time;

use gtk::prelude::*;
use gtk::{
    ApplicationWindow, Builder, Button, ComboBoxText, Dialog, Entry, Label, ListBox, ToggleButton,
};

macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

#[cfg(feature = "gtk_3_10")]
fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    db_init();

    let main_glade = include_str!("../res/main.glade");
    let builder = Builder::new_from_string(main_glade);
    let window: ApplicationWindow = builder.get_object("window").unwrap();
    window.set_title("MOMA");
    let listbox: ListBox = builder.get_object("list").unwrap();

    let total: Label = builder.get_object("total").unwrap();
    let main_currency: Label = builder.get_object("currency").unwrap();
    let wallet_select: ComboBoxText = builder.get_object("wallet").unwrap();
    let wallets = get_all_wallets();

    for wallet in wallets {
        wallet_select.append(wallet.id.to_string().as_str(), &wallet.name);
        if wallet.isdefault {
            wallet_select.set_active_id(wallet.id.to_string().as_str());
            gtk_wallet_refresh(wallet.id, &listbox, &total, &main_currency);
        }
    }

    wallet_select.connect_changed(
        clone!(listbox, main_currency, total => move |wallet_select| {
        let wallet = wallet_select.get_active_id().unwrap();
        gtk_wallet_refresh(wallet.parse::<i64>().unwrap(), &listbox, &total, &main_currency)

    }));

    let new_wallet_button: Button = builder.get_object("new_wallet").unwrap();
    new_wallet_button.connect_clicked(
        clone!(wallet_select, listbox, total, main_currency => move |_| {
        let wallet_builder = Builder::new_from_string(include_str!("../res/wallet.glade"));
        let dialog: Dialog = wallet_builder.get_object("wallet").unwrap();
        let name: Entry = wallet_builder.get_object("name").unwrap();
        let value: Entry = wallet_builder.get_object("value").unwrap();
        let default: ToggleButton = wallet_builder.get_object("default").unwrap();
        let currency: ComboBoxText = wallet_builder.get_object("currency").unwrap();
        let delete: Button = wallet_builder.get_object("delete").unwrap();
        delete.set_sensitive(false);

        let ret = dialog.run();
        if ret == -5 {
            let mut new_wallet = Wallet {
                id: 0,
                name: name.get_text().unwrap(),
                value: value.get_text().unwrap(),
                currency: currency.get_active_text().unwrap(),
                isdefault: default.get_active(),
                createdtime: time::get_time(),
            };

            let conn = db_connect();
            conn.execute(
                "INSERT INTO wallets (name, value, currency, isdefault, createdtime)
             VALUES (?1, ?2, ?3, ?4, ?5)",
                &[
                    &new_wallet.name,
                    &new_wallet.value,
                    &new_wallet.currency,
                    &new_wallet.isdefault,
                    &new_wallet.createdtime,
                ],
            ).unwrap();

            new_wallet.id = conn.last_insert_rowid();

            if new_wallet.isdefault {
                conn.execute(
                    "UPDATE wallets SET isdefault = 0 WHERE id != ?",
                    &[&new_wallet.id],
                ).unwrap();
            }
            wallet_select.append(new_wallet.id.to_string().as_str(),&new_wallet.name);
            wallet_select.set_active_id(new_wallet.id.to_string().as_str());
            gtk_wallet_refresh(new_wallet.id, &listbox, &total, &main_currency);
        }

        dialog.destroy();
    }),
    );

    let edit_wallet_button: Button = builder.get_object("edit_wallet").unwrap();
    edit_wallet_button.connect_clicked(clone!(wallet_select, listbox, main_currency => move |_| {
        let wallet_builder = Builder::new_from_string(include_str!("../res/wallet.glade"));
        let dialog: Dialog = wallet_builder.get_object("wallet").unwrap();
        let name: Entry = wallet_builder.get_object("name").unwrap();
        let value: Entry = wallet_builder.get_object("value").unwrap();
        let default: ToggleButton = wallet_builder.get_object("default").unwrap();
        let currency: ComboBoxText = wallet_builder.get_object("currency").unwrap();
        let wid = wallet_select.get_active_id().unwrap().parse::<i64>().unwrap();

        let conn = db_connect();
        conn.query_row(
        "SELECT name, value, currency, isdefault FROM wallets WHERE id = ?",
        &[&wid],
        |row| {
            let orig_name: String = row.get(0);
            let orig_value: String = row.get(1);
            let orig_currency: String = row.get(2);
            name.set_text(&orig_name);
            value.set_text(&orig_value);
            currency.set_active_id(orig_currency.as_str());
            let isdefault: bool = row.get(3);
            if isdefault {
                default.set_active(true);
            }
            },
        ).unwrap();

        let ret = dialog.run();

        if ret == -5 {
            let edit_wallet = Wallet {
                id: wid,
                name: name.get_text().unwrap(),
                value: value.get_text().unwrap(),
                currency: currency.get_active_text().unwrap(),
                isdefault: default.get_active(),
                createdtime: time::get_time(),
            };

            conn.execute(
                "UPDATE wallets SET name = ?1, value = ?2, currency = ?3, isdefault = ?4 WHERE id = ?5",
                &[
                    &edit_wallet.name,
                    &edit_wallet.value,
                    &edit_wallet.currency,
                    &edit_wallet.isdefault,
                    &wid,
                ],
            ).unwrap();

            if edit_wallet.isdefault {
                conn.execute(
                    "UPDATE wallets SET isdefault = 0 WHERE id != ?",
                    &[&wid],
                ).unwrap();
            }

            let active = wallet_select.get_active();
            wallet_select.insert(active, wid.to_string().as_str(), &edit_wallet.name);
            wallet_select.set_active(active);
            gtk_wallet_refresh(wid, &listbox, &total, &main_currency);
            ComboBoxTextExt::remove(&wallet_select, active+1);
        }

        if ret == -2 {
            let confirm_dialog = Dialog::new_with_buttons(Some("Delete?"), Some(&dialog), gtk::DialogFlags::MODAL,
                &[("No", 0), ("Yes", 1)]);

            let ret = confirm_dialog.run();
            if ret == 1 {
                conn.execute("DELETE FROM wallets WHERE id = ?",&[&wid]).unwrap();

                let active = wallet_select.get_active();
                wallet_select.set_active(0);
                ComboBoxTextExt::remove(&wallet_select, active);
                gtk_wallet_refresh(-1, &listbox, &total, &main_currency);
            }
            confirm_dialog.destroy();
        }

        dialog.destroy();
    }));

    let income_button: Button = builder.get_object("income").unwrap();
    let expenses_button: Button = builder.get_object("expenses").unwrap();
    income_button.connect_clicked(clone!(builder, listbox, wallet_select => move |_| {
        add_transaction(&builder, &listbox, &wallet_select, true);
    }));
    expenses_button.connect_clicked(move |_| {
        add_transaction(&builder, &listbox, &wallet_select, false);
    });

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    window.show_all();
    gtk::main();
}

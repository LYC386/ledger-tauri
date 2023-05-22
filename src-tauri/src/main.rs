// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod ledger_util;
use ethers_core::types::{Address, U256};
use ledger_util::Error;
#[tauri::command]
fn get_pk(num: &str) -> Result<String, String> {
    let path = format!("44'/60'/{}'/0/0", num);
    let (pk, address) = match ledger_util::get_pk(&path) {
        Ok(r) => r,
        Err(e) => match e {
            Error::ParsePathError => return Err("Invalid Path".into()),
            Error::LedgerError(e) => return Err(parse_ledger_error(e)),
        },
    };
    let result = format! {"PK:{}\nAddr:{}",pk,address};
    Ok(result)
}

#[tauri::command]
fn sign_data(num: &str, msg: &str, chain_id: &str) -> Result<String, String> {
    let path = format!("44'/60'/{}'/0/0", num);
    let chain_id: u64 = match chain_id.parse() {
        Ok(n) => n,
        Err(_) => return Err("Invalid chain ID".into()),
    };
    let (v, r, s) = match ledger_util::sign_message(&path, msg, chain_id) {
        Ok(r) => r,
        Err(e) => match e {
            Error::ParsePathError => return Err("Invalid Path".into()),
            Error::LedgerError(e) => return Err(parse_ledger_error(e)),
        },
    };
    let result = format!("v:{}\n,r:{}\n,s:{}", v, r, s);
    Ok(result)
}

#[tauri::command]
fn sign_tx(
    num: &str,
    chain_id: &str,
    value: &str,
    to: &str,
    nonce: &str,
    priority_fee: &str,
    max_fee: &str,
) -> Result<String, String> {
    let chain_id: u64 = match chain_id.parse() {
        Ok(n) => n,
        Err(_) => return Err("Invalid chain ID".into()),
    };
    let to: Address = match to.parse() {
        Ok(n) => n,
        Err(_) => return Err("Invalid recipient address".into()),
    };
    let value: f64 = match value.parse() {
        Ok(n) => n,
        Err(_) => return Err("Invalid value".into()),
    };
    let nonce: U256 = match nonce.parse() {
        Ok(n) => n,
        Err(_) => return Err("Invalid nonce value".into()),
    };
    let priority_fee: U256 = match priority_fee.parse() {
        Ok(n) => n,
        Err(_) => return Err("Invalid priority fee value".into()),
    };
    let max_fee: U256 = match max_fee.parse() {
        Ok(n) => n,
        Err(_) => return Err("Invalid max_fee value".into()),
    };
    let path = format!("44'/60'/{}'/0/0", num);

    let hex_signed_tx = match ledger_util::sign_tx(
        to,
        &path,
        value,
        nonce,
        priority_fee,
        max_fee,
        None,
        chain_id,
        None,
    ) {
        Ok(r) => r,
        Err(e) => match e {
            Error::ParsePathError => return Err("Invalid Path".into()),
            Error::LedgerError(e) => return Err(parse_ledger_error(e)),
        },
    };
    Ok(format!("Signed tx: {}", hex_signed_tx))
}

fn parse_ledger_error(e: ledger::Error) -> String {
    match e {
        ledger::Error::Apdu(s) => {
            if s == "[APDU_CODE_CONDITIONS_NOT_SATISFIED] Conditions of use not satisfied" {
                return "Cancelled".into();
            } else {
                return "Please open Eth app on Ledger".into();
            }
        }
        ledger::Error::DeviceNotFound => return "Ledger not found".into(),
        _ => return "Error".into(),
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_pk, sign_data, sign_tx])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod ledger_util;
use ethers_core::types::{Address, U256};
use ledger_transport_hid::LedgerHIDError;
use ledger_util::{new_ledger, Error};
#[tauri::command]
fn get_pk(num: &str) -> Result<String, String> {
    let ledger = match new_ledger() {
        Ok(l) => l,
        Err(e) => return Err(e.to_string()),
    };

    let path = format!("44'/60'/{}'/0/0", num);
    let (pk, address) = match ledger_util::get_pk(&path, &ledger) {
        Ok(r) => r,
        Err(e) => match e {
            Error::ParsePathError => return Err("Invalid Path".into()),
            Error::LedgerError(e) => {
                return Err(parse_ledger_error(e));
            }
        },
    };
    let result = format! {"PK:{}\nAddr:{}",pk,address};
    Ok(result)
}

#[tauri::command]
fn sign_data(num: &str, msg: &str, chain_id: &str) -> Result<String, String> {
    let ledger = match new_ledger() {
        Ok(l) => l,
        Err(e) => return Err(e.to_string()),
    };
    let path = format!("44'/60'/{}'/0/0", num);
    let chain_id: u64 = match chain_id.parse() {
        Ok(n) => n,
        Err(_) => return Err("Invalid chain ID".into()),
    };
    let (v, r, s) = match ledger_util::sign_message(&path, msg, chain_id, &ledger) {
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
    gas: Option<&str>,
    priority_fee: &str,
    max_fee: &str,
    data: Option<Vec<u8>>,
) -> Result<String, String> {
    let ledger = match new_ledger() {
        Ok(l) => l,
        Err(e) => return Err(e.to_string()),
    };
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
    let nonce: U256 = match U256::from_dec_str(nonce) {
        Ok(n) => n,
        Err(_) => return Err("Invalid nonce value".into()),
    };
    let gas: Option<U256> = match gas {
        Some(n) => match U256::from_dec_str(n) {
            Ok(g) => Some(g),
            Err(_) => return Err("Invalid gas value".into()),
        },
        None => None,
    };
    let priority_fee: U256 = match U256::from_dec_str(priority_fee) {
        Ok(n) => n,
        Err(_) => return Err("Invalid priority fee value".into()),
    };
    let max_fee: U256 = match U256::from_dec_str(max_fee) {
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
        gas,
        chain_id,
        data,
        &ledger,
    ) {
        Ok(r) => r,
        Err(e) => match e {
            Error::ParsePathError => return Err("Invalid Path".into()),
            Error::LedgerError(e) => return Err(parse_ledger_error(e)),
        },
    };
    Ok(format!("Signed tx: {}", hex_signed_tx))
}

fn parse_ledger_error(e: LedgerHIDError) -> String {
    match e {
        LedgerHIDError::DeviceNotFound => return "Ledger not found".into(),
        LedgerHIDError::Comm(r) => return r.into(),
        _ => return e.to_string(),
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_pk, sign_data, sign_tx])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

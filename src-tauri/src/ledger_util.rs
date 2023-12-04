use ethers_core::types::{Address, Eip1559TransactionRequest, Signature, U256};
use ethers_core::utils::{hex, parse_ether};
use ledger_transport::APDUCommand;
use ledger_transport_hid::hidapi::HidApi;
use ledger_transport_hid::LedgerHIDError;
use ledger_transport_hid::TransportNativeHID;
use std::fmt;
use std::num::ParseIntError;
use std::str;
#[derive(Debug)]
pub enum Error {
    ParsePathError,
    LedgerError(LedgerHIDError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::LedgerError(e) => format!("Ledger error: {}", e),
            Self::ParsePathError => format!("Unable to parse path"),
        };
        write!(f, "{s}")
    }
}

impl From<LedgerHIDError> for Error {
    fn from(value: LedgerHIDError) -> Self {
        Self::LedgerError(value)
    }
}

// must drop existing TransportNativeHID before calling
pub fn new_ledger() -> Result<TransportNativeHID, Error> {
    let h = HidApi::new().unwrap();
    Ok(TransportNativeHID::new(&h)?)
}

fn parse_bip32_path(path: &str) -> Result<Vec<u8>, ParseIntError> {
    let mut result = Vec::<u8>::new();
    let v_path: Vec<&str> = path.split("/").collect();
    for path_element in v_path {
        let element: Vec<&str> = path_element.split("'").collect();
        if element.len() == 1 {
            let index = u32::from_str_radix(element[0], 10)?;
            let index = index.to_be_bytes();
            result.append(&mut index.to_vec());
        } else {
            let index = u32::from_str_radix(element[0], 10)?;
            let n = 0x80000000u32;
            let index = index | n;
            let index = index.to_be_bytes();
            result.append(&mut index.to_vec());
        }
    }
    Ok(result)
}

pub fn get_pk(path: &str, ledger: &TransportNativeHID) -> Result<(String, String), Error> {
    let r = match parse_bip32_path(path) {
        Ok(r) => r,
        Err(_) => return Err(Error::ParsePathError),
    };
    let length: u8 = (r.len() + 1).try_into().unwrap();
    let data = [vec![(length - 1) / 4], r].concat();

    let command = APDUCommand {
        cla: 0xe0,
        ins: 0x02,
        p1: 0x00,
        p2: 0x00,
        data,
    };

    let r = ledger.exchange(&command)?;

    let result = r.apdu_data();
    if result.len() == 0 {
        if get_opened_app(&ledger)?.starts_with("Ethereum") {
            return Err(LedgerHIDError::Comm("Canceled").into());
        } else {
            return Err(LedgerHIDError::Comm("Please open Ethereum app on Ledger").into());
        }
    }
    let offset = 1 + result[0];
    let start = usize::try_from(offset + 1).unwrap();
    let end = usize::try_from(offset + 1 + result[usize::try_from(offset).unwrap()]).unwrap();
    let address = &result[start..end];
    let pk = &result[1..usize::try_from(offset).unwrap()];
    let pk = hex::encode(pk);
    let address = format!("0x{}", str::from_utf8(address).unwrap());
    Ok((pk, address))
}

// sign msg without eip-191 prefix
pub fn sign_message(
    path: &str,
    msg: &str,
    chain_id: u64,
    ledger: &TransportNativeHID,
) -> Result<(String, String, String), Error> {
    //let sign_message = &b"\x19Ethereum Signed Message:\n"[..];
    let msg = msg.as_bytes();
    let msg_len = msg.len();
    let b_path = match parse_bip32_path(path) {
        Ok(r) => r,
        Err(_) => return Err(Error::ParsePathError),
    };
    let bip32_num: u8 = (b_path.len() / 4).try_into().unwrap();

    // divide msg if too long
    let mut result = Vec::new();
    let mut first_chunk = true;
    let data_chunk_iter = msg.chunks(230);
    for chunk in data_chunk_iter {
        if first_chunk {
            let d = [&u32::try_from(msg_len).unwrap().to_be_bytes()[..], chunk].concat();
            let data = [&[bip32_num][..], &b_path, &d].concat();
            let command = APDUCommand {
                cla: 0xe0,
                ins: 0x08,
                p1: 0x00,
                p2: 0x00,
                data,
            };
            let ledger_return = ledger.exchange(&command)?;
            result = ledger_return.apdu_data().to_vec();
            first_chunk = false;
        } else {
            let command = APDUCommand {
                cla: 0xe0,
                ins: 0x08,
                p1: 0x80,
                p2: 0x00,
                data: chunk,
            };
            let ledger_return = ledger.exchange(&command)?;
            result = ledger_return.apdu_data().to_vec();
        }
    }

    if result.len() == 0 {
        if get_opened_app(&ledger)? == "Ethereum" {
            return Err(LedgerHIDError::Comm("Canceled").into());
        } else {
            return Err(LedgerHIDError::Comm("Please open Ethereum app on Ledger").into());
        }
    }
    let v: u64 = result[0].try_into().unwrap();
    let ecc_parity: u64;
    if (chain_id * 2 + 35) + 1 > 255 {
        ecc_parity = v - ((chain_id * 2 + 35) % 256);
    } else {
        ecc_parity = (v + 1) % 2;
    }
    let v = ecc_parity;
    let r = result[1..33].to_vec();
    let s = result[33..65].to_vec();
    //println!("{v}");
    Ok((v.to_string(), hex::encode(r), hex::encode(s)))
}

pub fn sign_tx(
    to: Address,
    path: &str,
    eth_amount: f64,
    nonce: U256,
    max_priority_fee_per_gas: U256,
    max_fee_per_gas: U256,
    gas: Option<U256>,
    chain_id: u64,
    hex_data: Option<Vec<u8>>,
    ledger: &TransportNativeHID,
) -> Result<String, Error> {
    let mut tx = Eip1559TransactionRequest::new();
    let amount = parse_ether(eth_amount).unwrap();
    let gas = match gas {
        Some(g) => g,
        None => U256::from(21000u128),
    };
    let data = match hex_data {
        Some(d) => d,
        None => Vec::<u8>::default(),
    };

    tx = tx
        .to(to)
        .value(amount)
        .nonce(nonce)
        .chain_id(chain_id)
        .gas(gas)
        .max_fee_per_gas(max_fee_per_gas)
        .max_priority_fee_per_gas(max_priority_fee_per_gas)
        .data(data);

    let mut encoded_tx = tx.rlp().to_vec();
    encoded_tx = [&[0x02u8][..], &encoded_tx].concat();
    let b_path = match parse_bip32_path(path) {
        Ok(r) => r,
        Err(_) => return Err(Error::ParsePathError),
    };
    let bip32_num: u8 = (b_path.len() / 4).try_into().unwrap();
    let data = [&[bip32_num][..], &b_path, &encoded_tx].concat();

    // divide data if too long
    let mut result = Vec::new();
    let mut first_chunk = true;
    let data_chunk_iter = data.chunks(255);
    for chunk in data_chunk_iter {
        if first_chunk {
            let command = APDUCommand {
                cla: 0xe0,
                ins: 0x04,
                p1: 0x00,
                p2: 0x00,
                data: chunk,
            };
            let r = ledger.exchange(&command)?;
            result = r.apdu_data().to_vec();
            first_chunk = false;
        } else {
            let command = APDUCommand {
                cla: 0xe0,
                ins: 0x04,
                p1: 0x80,
                p2: 0x00,
                data: chunk,
            };
            let r = ledger.exchange(&command)?;
            result = r.apdu_data().to_vec();
        }
    }

    if result.len() == 0 {
        if get_opened_app(&ledger)? == "Ethereum" {
            return Err(LedgerHIDError::Comm("Canceled").into());
        } else {
            return Err(LedgerHIDError::Comm("Please open Ethereum app on Ledger").into());
        }
    }
    let v: u64 = result[0].try_into().unwrap();
    let r = &result[1..33];
    let s = &result[33..65];

    let sig = Signature {
        r: U256::from_big_endian(r),
        s: U256::from_big_endian(s),
        v,
    };
    let tx_signed = tx.rlp_signed(&sig);
    let hex_tx_signed = hex::encode(&tx_signed);
    Ok(hex_tx_signed)
}

fn get_opened_app(ledger: &TransportNativeHID) -> Result<String, Error> {
    let command = APDUCommand {
        cla: 0xb0,
        ins: 0x01,
        p1: 0x00,
        p2: 0x00,
        data: Vec::new(),
    };
    let r = ledger.exchange(&command)?;
    let result = r.apdu_data();
    if result.len() == 0 {
        return Err(LedgerHIDError::Comm("Device Locked").into());
    }
    let len: usize = result[1].into();
    Ok(String::from_utf8(result[2..2 + len].to_vec()).unwrap())
}

// fn open_app(app: &str, ledger: &TransportNativeHID) -> Result<(), Error> {
//     let data = app.to_string().into_bytes();
//     let command = APDUCommand {
//         cla: 0xe0,
//         ins: 0xd8,
//         p1: 0x00,
//         p2: 0x00,
//         data,
//     };

//     let _ = ledger.exchange(&command).unwrap();
//     Ok(())
// }

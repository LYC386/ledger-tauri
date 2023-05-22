use ethers_core::types::{Address, Eip1559TransactionRequest, Signature, U256};
use ethers_core::utils::{hex, parse_ether};
use ledger::{ApduCommand, LedgerApp};
use std::fmt;
use std::num::ParseIntError;
use std::str;
#[derive(Debug)]
pub enum Error {
    ParsePathError,
    LedgerError(ledger::Error),
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

impl From<ledger::Error> for Error {
    fn from(value: ledger::Error) -> Self {
        Self::LedgerError(value)
    }
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

pub fn get_pk(path: &str) -> Result<(String, String), Error> {
    let r = match parse_bip32_path(path) {
        Ok(r) => r,
        Err(_) => return Err(Error::ParsePathError),
    };
    let length: u8 = (r.len() + 1).try_into().unwrap();
    let data = [vec![(length - 1) / 4], r].concat();
    let ledger = LedgerApp::new()?;
    //ledger.set_logging(true);

    let command = ApduCommand {
        cla: 0xe0,
        ins: 0x02,
        p1: 0x00,
        p2: 0x00,
        length: length,
        data: data,
    };
    let result = ledger.exchange(command)?.data;
    let offset = 1 + result[0];
    let start = usize::try_from(offset + 1).unwrap();
    let end = usize::try_from(offset + 1 + result[usize::try_from(offset).unwrap()]).unwrap();
    let address = &result[start..end];
    let pk = &result[1..usize::try_from(offset).unwrap()];
    let pk = hex::encode(pk);
    let address = format!("0x{}", str::from_utf8(address).unwrap());
    LedgerApp::close();
    Ok((pk, address))
}

pub fn sign_message(
    path: &str,
    msg: &str,
    chain_id: u64,
) -> Result<(String, String, String), Error> {
    //let sign_message = &b"\x19Ethereum Signed Message:\n"[..];
    let msg = msg.as_bytes();
    let b_path = match parse_bip32_path(path) {
        Ok(r) => r,
        Err(_) => return Err(Error::ParsePathError),
    };
    let bip32_num: u8 = (b_path.len() / 4).try_into().unwrap();
    let encoded_tx = [&u32::try_from(msg.len()).unwrap().to_be_bytes()[..], msg].concat();
    let length: u8 = (b_path.len() + 1 + encoded_tx.len()).try_into().unwrap();
    let data = [&[bip32_num][..], &b_path, &encoded_tx].concat();
    //println!("{:02x?}", data);
    let ledger = LedgerApp::new()?;
    let command = ApduCommand {
        cla: 0xe0,
        ins: 0x08,
        p1: 0x00,
        p2: 0x00,
        length: length,
        data: data,
    };
    let result = ledger.exchange(command)?.data;
    //println!("{}", hex::encode(&result));
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
    hex_data: Option<&str>,
    //hexdescriptor: Option<&str>,
) -> Result<String, Error> {
    let mut tx = Eip1559TransactionRequest::new();
    let amount = parse_ether(eth_amount).unwrap();
    let gas = match gas {
        Some(g) => g,
        None => U256::from(21000u128),
    };
    let data = match hex_data {
        Some(d) => hex::decode(d).unwrap(),
        None => b"".to_vec(),
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
    let ledger = LedgerApp::new()?;
    let b_path = match parse_bip32_path(path) {
        Ok(r) => r,
        Err(_) => return Err(Error::ParsePathError),
    };
    let bip32_num: u8 = (b_path.len() / 4).try_into().unwrap();
    let length: u8 = (b_path.len() + 1 + encoded_tx.len()).try_into().unwrap();
    let data = [&[bip32_num][..], &b_path, &encoded_tx].concat();
    let command = ApduCommand {
        cla: 0xe0,
        ins: 0x04,
        p1: 0x00,
        p2: 0x00,
        length: length,
        data: data,
    };
    let result = ledger.exchange(command)?.data;
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

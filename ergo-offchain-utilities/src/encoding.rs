/// This file holds various functions related to encoding/serialization of values that are relevant
/// to the oracle core.
use crate::{ErgoAddressString, P2PKAddressString, P2SAddressString};
use base16;
use blake2b_simd::Params;
use ergo_lib::ast::{CollPrim, Constant, ConstantColl, ConstantVal};
use ergo_lib::chain::address::{Address, AddressEncoder, NetworkPrefix};
use ergo_lib::serialization::SigmaSerializable;
use ergo_lib::ErgoTree;
use std::fmt::{Debug, Display};
use std::str;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, EncodingError<String>>;

#[derive(Error, Debug)]
pub enum EncodingError<T: Debug + Display> {
    #[error("Failed to serialize: {0}")]
    FailedToSerialize(T),
    #[error("Failed to deserialize: {0}")]
    FailedToDeserialize(T),
}

/// Takes the blake2b hash of a String, then converted into/represented as hex as a String
pub fn string_to_blake2b_hash(b: String) -> Result<String> {
    let mut params = Params::new();
    params.hash_length(32);
    let a = params.hash(&decode_hex(&b)?).to_hex().to_string();
    Ok(a)
}

/// Serialize a `i32` Int value into a hex-encoded string to be used inside of a register for a box
pub fn serialize_int(i: i32) -> String {
    let constant: Constant = i.into();
    constant.base16_str()
}

/// Serialize a `i64` Long value into a hex-encoded string to be used inside of a register for a box
pub fn serialize_long(i: i64) -> String {
    let constant: Constant = i.into();
    constant.base16_str()
}

/// Serialize a `String` value into a hex-encoded string to be used inside of a register for a box
pub fn serialize_string(s: &String) -> String {
    let b = convert_to_signed_bytes(&s.clone().into_bytes());
    let constant: Constant = b.into();
    constant.base16_str()
}

/// Decodes a hex-encoded string into bytes and then serializes it into a properly formatted hex-encoded string to be used inside of a register for a box
pub fn serialize_hex_encoded_string(s: &String) -> Result<String> {
    let b = decode_hex(s)?;
    let constant: Constant = convert_to_signed_bytes(&b).into();
    Ok(constant.base16_str())
}

/// Deserialize a hex-encoded `i32` Int inside of a `Constant` acquired from a register of an `ErgoBox`
pub fn deserialize_int(c: &Constant) -> Result<i32> {
    match &c.v {
        ConstantVal::Int(i) => return Ok(i.clone()),
        _ => return Err(EncodingError::FailedToDeserialize(c.base16_str())),
    };
}

/// Deserialize a hex-encoded `i64` Long inside of a `Constant` acquired from a register of an `ErgoBox`
pub fn deserialize_long(c: &Constant) -> Result<i64> {
    // Eventually use &c.v.try_extract_from()
    match &c.v {
        ConstantVal::Long(i) => return Ok(i.clone()),
        _ => return Err(EncodingError::FailedToDeserialize(c.base16_str())),
    };
}

/// Deserialize a String which is inside of a `Constant` acquired from a register of an `ErgoBox`
pub fn deserialize_string(c: &Constant) -> Result<String> {
    let byte_array: Result<Vec<u8>> = match &c.v {
        ConstantVal::Coll(ConstantColl::Primitive(CollPrim::CollByte(ba))) => {
            Ok(convert_to_unsigned_bytes(ba))
        }
        _ => Err(EncodingError::FailedToDeserialize(c.base16_str())),
    };
    Ok(str::from_utf8(&byte_array?)
        .map_err(|_| EncodingError::FailedToDeserialize(c.base16_str()))?
        .to_string())
}

/// Deserialize a hex-encoded String which is inside of a `Constant` acquired from a register of an `ErgoBox`.
pub fn deserialize_hex_encoded_string(c: &Constant) -> Result<String> {
    let byte_array: Result<Vec<u8>> = match &c.v {
        ConstantVal::Coll(ConstantColl::Primitive(CollPrim::CollByte(ba))) => {
            Ok(convert_to_unsigned_bytes(ba))
        }
        _ => Err(EncodingError::FailedToDeserialize(c.base16_str())),
    };
    Ok(base16::encode_lower(&byte_array?))
}

/// Acquire the `ErgoTree` of the P2S Base58 String.
pub fn serialize_p2s_to_ergo_tree(p2s_address: P2SAddressString) -> Result<ErgoTree> {
    let encoder = AddressEncoder::new(NetworkPrefix::Mainnet);
    let address = encoder
        .parse_address_from_str(&p2s_address)
        .map_err(|_| EncodingError::FailedToSerialize(p2s_address.clone()))?;
    ErgoTree::sigma_parse_bytes(address.content_bytes())
        .map_err(|_| EncodingError::FailedToSerialize(p2s_address.clone()))
}

/// Acquire the Base58 encoded P2S Address from a `ErgoTree`
pub fn deserialize_p2s_from_ergo_tree(ergo_tree: ErgoTree) -> P2SAddressString {
    let address = Address::P2S(ergo_tree.sigma_serialise_bytes());
    let encoder = AddressEncoder::new(NetworkPrefix::Mainnet);
    encoder.address_to_str(&address)
}

/// Deserialize ErgoTree inside of a `Constant` acquired from a register of an `ErgoBox` into a P2S Base58 String.
pub fn deserialize_ergo_tree_constant(c: &Constant) -> Result<P2SAddressString> {
    let byte_array: Result<Vec<u8>> = match &c.v {
        ConstantVal::Coll(ConstantColl::Primitive(CollPrim::CollByte(ba))) => {
            Ok(convert_to_unsigned_bytes(ba))
        }
        _ => Err(EncodingError::FailedToDeserialize(c.base16_str())),
    };

    let address = Address::P2S(byte_array?);
    let encoder = AddressEncoder::new(NetworkPrefix::Mainnet);

    Ok(encoder.address_to_str(&address))
}

/// Convert Vec<i8> to Vec<u8>
fn convert_to_unsigned_bytes(bytes: &Vec<i8>) -> Vec<u8> {
    bytes.iter().map(|x| x.clone() as u8).collect()
}

/// Convert Vec<u8> to Vec<i8>
fn convert_to_signed_bytes(bytes: &Vec<u8>) -> Vec<i8> {
    bytes.iter().map(|x| x.clone() as i8).collect()
}

/// Takes an Ergo address (either P2PK or P2S) as a Base58 String and returns
/// the `ErgoTree` if it is a valid address.
pub fn address_string_to_ergo_tree(address_str: &ErgoAddressString) -> Result<ErgoTree> {
    let encoder = AddressEncoder::new(NetworkPrefix::Mainnet);
    let address = encoder
        .parse_address_from_str(address_str)
        .map_err(|_| EncodingError::FailedToSerialize(address_str.to_string()))?;
    let ergo_tree = ErgoTree::sigma_parse_bytes(address.content_bytes())
        .map_err(|_| EncodingError::FailedToSerialize(address_str.to_string()))?;
    Ok(ergo_tree)
}

/// Decodes a hex-encoded string into bytes
fn decode_hex(s: &String) -> Result<Vec<u8>> {
    if let Ok(b) = base16::decode(s) {
        return Ok(b);
    } else {
        return Err(EncodingError::FailedToSerialize(s.clone()));
    }
}

/// Convert from Erg to nanoErg
pub fn erg_to_nanoerg(erg_amount: f64) -> u64 {
    (erg_amount * 1000000000 as f64) as u64
}

/// Convert from nanoErg to Erg
pub fn nanoerg_to_erg(nanoerg_amount: u64) -> f64 {
    (nanoerg_amount as f64) / (1000000000 as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_serialization_test() {
        let l: i64 = 255;
        let ser_l: String = serialize_long(l);
        let constant_l: Constant = l.into();
        assert_eq!(ser_l, "05fe03".to_string());
        assert_eq!(l, deserialize_long(&constant_l).unwrap());

        assert_eq!(
            serialize_string(&"Oracle Pools".to_string()),
            "0e0c4f7261636c6520506f6f6c73".to_string()
        );
    }

    #[test]
    fn string_serialization_test() {
        let s: String = "Oracle Pools".to_string();
        let a = s.clone().into_bytes();
        let b: Vec<i8> = a.iter().map(|c| c.clone() as i8).collect();
        let constant: Constant = b.into();

        assert_eq!(s, deserialize_string(&constant).unwrap());
        assert_eq!(
            serialize_string(&s),
            "0e0c4f7261636c6520506f6f6c73".to_string()
        );
    }

    #[test]
    fn erg_conv_is_valid() {
        assert_eq!((1 as f64), nanoerg_to_erg(1000000000));
        assert_eq!((1.23 as f64), nanoerg_to_erg(1230000000));

        assert_eq!(1000000000, erg_to_nanoerg(1 as f64));
        assert_eq!(erg_to_nanoerg(3.64), 3640000000);
        assert_eq!(erg_to_nanoerg(0.64), 640000000);
        assert_eq!(erg_to_nanoerg(0.0064), 6400000);
        assert_eq!(erg_to_nanoerg(0.000000064), 64);
        assert_eq!(erg_to_nanoerg(0.000000001), 1);
    }
}

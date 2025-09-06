// use crate::layouts::{ArrayDef, FieldDef, LayoutJson};
use crate::layouts::{ArrayDef, FieldDef};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseErr {
    #[error("out of bounds: need {need} at {at}, data_len={len}")]
    Oob { at: usize, need: usize, len: usize },
    #[error("unknown type: {0}")]
    UnknownType(String),
}

fn need<'a>(data: &'a [u8], at: usize, need: usize) -> Result<&'a [u8], ParseErr> {
    if at + need <= data.len() {
        Ok(&data[at..at + need])
    } else {
        Err(ParseErr::Oob {
            at,
            need,
            len: data.len(),
        })
    }
}

// Little-endian readers
pub fn read_u8(data: &[u8], at: usize) -> Result<u8, ParseErr> {
    Ok(*need(data, at, 1)?.first().unwrap())
}
pub fn read_u16(data: &[u8], at: usize) -> Result<u16, ParseErr> {
    Ok(u16::from_le_bytes(need(data, at, 2)?.try_into().unwrap()))
}
pub fn read_u32(data: &[u8], at: usize) -> Result<u32, ParseErr> {
    Ok(u32::from_le_bytes(need(data, at, 4)?.try_into().unwrap()))
}
pub fn read_i32(data: &[u8], at: usize) -> Result<i32, ParseErr> {
    Ok(i32::from_le_bytes(need(data, at, 4)?.try_into().unwrap()))
}
pub fn read_u64(data: &[u8], at: usize) -> Result<u64, ParseErr> {
    Ok(u64::from_le_bytes(need(data, at, 8)?.try_into().unwrap()))
}
pub fn read_i64(data: &[u8], at: usize) -> Result<i64, ParseErr> {
    Ok(i64::from_le_bytes(need(data, at, 8)?.try_into().unwrap()))
}
pub fn read_u128(data: &[u8], at: usize) -> Result<u128, ParseErr> {
    Ok(u128::from_le_bytes(need(data, at, 16)?.try_into().unwrap()))
}
pub fn read_i128(data: &[u8], at: usize) -> Result<i128, ParseErr> {
    Ok(i128::from_le_bytes(need(data, at, 16)?.try_into().unwrap()))
}
pub fn read_pubkey(data: &[u8], at: usize) -> Result<[u8; 32], ParseErr> {
    Ok(need(data, at, 32)?.try_into().unwrap())
}

pub fn read_scalar(data: &[u8], f: &FieldDef) -> Result<ScalarValue, ParseErr> {
    match f.ty.as_str() {
        "u8" => Ok(ScalarValue::U8(read_u8(data, f.offset)?)),
        "u16" => Ok(ScalarValue::U16(read_u16(data, f.offset)?)),
        "u32" => Ok(ScalarValue::U32(read_u32(data, f.offset)?)),
        "i32" => Ok(ScalarValue::I32(read_i32(data, f.offset)?)),
        "u64" => Ok(ScalarValue::U64(read_u64(data, f.offset)?)),
        "i64" => Ok(ScalarValue::I64(read_i64(data, f.offset)?)),
        "u128" => Ok(ScalarValue::U128(read_u128(data, f.offset)?)),
        "i128" => Ok(ScalarValue::I128(read_i128(data, f.offset)?)),
        "Pubkey" => Ok(ScalarValue::Pubkey(read_pubkey(data, f.offset)?)),
        other => Err(ParseErr::UnknownType(other.to_string())),
    }
}

#[derive(Debug, Clone)]
pub enum ScalarValue {
    U8(u8),
    U16(u16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    U128(u128),
    I128(i128),
    Pubkey([u8; 32]),
}

#[derive(Debug, Clone)]
pub struct TickLike {
    pub tick_or_bin_index: i32,
    pub liquidity_gross: u128,
    pub liquidity_net_or_y: i128, // allow i128 (CLMM net) or treat as y for DLMM
    pub at: usize,                // byte offset of this element
}

// decode a fixed array of entries (tick array / bins)
pub fn parse_array_entries(
    data: &[u8],
    a: &ArrayDef,
) -> Result<Vec<Vec<(String, ScalarValue)>>, ParseErr> {
    let mut out = Vec::with_capacity(a.count);
    for i in 0..a.count {
        let base = a.start_offset + i * a.element_size;
        let mut row = Vec::with_capacity(a.fields.len());
        for fd in &a.fields {
            let mut fd2 = fd.clone();
            fd2.offset = base + fd.offset;
            let v = read_scalar(data, &fd2)?;
            row.push((fd.name.clone(), v));
        }
        out.push(row);
    }
    Ok(out)
}

//! Contains a few ways to style numbers. At the time of writing, Minecraft only
//! uses this for rendering scoreboard objectives.

#[cfg(feature = "azalea-buf")]
use std::io::{self, Cursor, Read, Write};

#[cfg(feature = "azalea-buf")]
use azalea_buf::{AzBuf, BufReadError};
#[cfg(feature = "azalea-buf")]
use azalea_registry::builtin::NumberFormatKind;
use simdnbt::owned::Nbt;

use crate::FormattedText;

#[derive(Clone, Debug, PartialEq)]
pub enum NumberFormat {
    Blank,
    Styled { style: Nbt },
    Fixed { value: FormattedText },
}

#[cfg(feature = "azalea-buf")]
impl AzBuf for NumberFormat {
    fn azalea_read(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        let kind = NumberFormatKind::azalea_read(buf)?;
        match kind {
            NumberFormatKind::Blank => Ok(NumberFormat::Blank),
            NumberFormatKind::Styled => {
                // FIX: simdnbt 0.10 не принимает Short(2) как root-тег.
                // Пробуем стандартный read. Если падает — пропускаем NBT.
                // ВАЖНО: simdnbt::owned::read уже съел первый байт (тип тега),
                // поэтому мы делаем свою проверку ДО вызова read.
                let style = read_nbt_root_safe(buf)?;
                Ok(NumberFormat::Styled { style })
            }
            NumberFormatKind::Fixed => Ok(NumberFormat::Fixed {
                value: FormattedText::azalea_read(buf)?,
            }),
        }
    }
    fn azalea_write(&self, buf: &mut impl Write) -> io::Result<()> {
        match self {
            NumberFormat::Blank => NumberFormatKind::Blank.azalea_write(buf)?,
            NumberFormat::Styled { style } => {
                NumberFormatKind::Styled.azalea_write(buf)?;
                style.write(buf);
            }
            NumberFormat::Fixed { value } => {
                NumberFormatKind::Fixed.azalea_write(buf)?;
                value.azalea_write(buf)?;
            }
        }
        Ok(())
    }
}

#[cfg(feature = "azalea-buf")]
fn io_err(e: io::Error) -> BufReadError {
    BufReadError::Io { source: e }
}

/// Читает NBT-тег из буфера, корректно сдвигая курсор для всех типов
/// корневых тегов. Возвращает `Nbt::None` для не-Compound тегов.
#[cfg(feature = "azalea-buf")]
fn read_nbt_root_safe(buf: &mut Cursor<&[u8]>) -> Result<Nbt, BufReadError> {
    let mut type_byte = [0u8; 1];
    buf.read_exact(&mut type_byte).map_err(io_err)?;
    let root_type = type_byte[0];

    match root_type {
        // Compound (10) — стандартный случай. Откатываемся на 1 байт
        // и читаем целиком через simdnbt::owned::read.
        10 => {
            buf.set_position(buf.position() - 1);
            simdnbt::owned::read(buf)
                .map_err(|e| BufReadError::Custom(format!("NBT read error: {e}")))
        }

        // Short (2) — читаем 2 байта, возвращаем Nbt::None
        2 => {
            let mut skip = [0u8; 2];
            buf.read_exact(&mut skip).map_err(io_err)?;
            Ok(Nbt::None)
        }
        // Byte (1)
        1 => {
            let mut skip = [0u8; 1];
            buf.read_exact(&mut skip).map_err(io_err)?;
            Ok(Nbt::None)
        }
        // Int (3)
        3 => {
            let mut skip = [0u8; 4];
            buf.read_exact(&mut skip).map_err(io_err)?;
            Ok(Nbt::None)
        }
        // Long (4)
        4 => {
            let mut skip = [0u8; 8];
            buf.read_exact(&mut skip).map_err(io_err)?;
            Ok(Nbt::None)
        }
        // Float (5)
        5 => {
            let mut skip = [0u8; 4];
            buf.read_exact(&mut skip).map_err(io_err)?;
            Ok(Nbt::None)
        }
        // Double (6)
        6 => {
            let mut skip = [0u8; 8];
            buf.read_exact(&mut skip).map_err(io_err)?;
            Ok(Nbt::None)
        }
        // ByteArray (7)
        7 => {
            let len = read_i32(buf)?;
            if len > 0 {
                let mut skip = vec![0u8; len as usize];
                buf.read_exact(&mut skip).map_err(io_err)?;
            }
            Ok(Nbt::None)
        }
        // String (8)
        8 => {
            let _s: String = String::azalea_read(buf)?;
            Ok(Nbt::None)
        }
        // List (9) — только заголовок
        9 => {
            let mut inner_type = [0u8; 1];
            buf.read_exact(&mut inner_type).map_err(io_err)?;
            let _len = read_i32(buf)?;
            Ok(Nbt::None)
        }
        // IntArray (11)
        11 => {
            let len = read_i32(buf)?;
            if len > 0 {
                let mut skip = vec![0u8; len as usize * 4];
                buf.read_exact(&mut skip).map_err(io_err)?;
            }
            Ok(Nbt::None)
        }
        // LongArray (12)
        12 => {
            let len = read_i32(buf)?;
            if len > 0 {
                let mut skip = vec![0u8; len as usize * 8];
                buf.read_exact(&mut skip).map_err(io_err)?;
            }
            Ok(Nbt::None)
        }
        // End (0) или неизвестный
        other => {
            eprintln!("[NBT] Unexpected root tag type: {other}, returning Nbt::None");
            Ok(Nbt::None)
        }
    }
}

#[cfg(feature = "azalea-buf")]
fn read_i32(buf: &mut Cursor<&[u8]>) -> Result<i32, BufReadError> {
    let mut b = [0u8; 4];
    buf.read_exact(&mut b).map_err(io_err)?;
    Ok(i32::from_be_bytes(b))
}

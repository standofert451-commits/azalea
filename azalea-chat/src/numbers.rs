//! Contains a few ways to style numbers. At the time of writing, Minecraft only
//! uses this for rendering scoreboard objectives.

#[cfg(feature = "azalea-buf")]
use std::io::{self, Cursor, Read, Write};

#[cfg(feature = "azalea-buf")]
use azalea_buf::{AzBuf, BufReadError};
#[cfg(feature = "azalea-buf")]
use azalea_registry::builtin::NumberFormatKind;
use simdnbt::owned::{Nbt, NbtCompound};

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
                style.azalea_write(buf)?;
            }
            NumberFormat::Fixed { value } => {
                NumberFormatKind::Fixed.azalea_write(buf)?;
                value.azalea_write(buf)?;
            }
        }
        Ok(())
    }
}

/// Обёртка для ошибок io в BufReadError::Io (struct variant).
#[cfg(feature = "azalea-buf")]
fn io_err(e: io::Error) -> BufReadError {
    BufReadError::Io { source: e }
}

/// Пустой NBT compound как Nbt.
#[cfg(feature = "azalea-buf")]
fn empty_nbt() -> Nbt {
    Nbt::Compound(NbtCompound::new())
}

/// Читает NBT-тег из буфера, корректно сдвигая курсор для всех типов
/// корневых тегов, включая Short(2). Возвращает пустой compound для
/// не-Compound тегов, потому что стиль можно проигнорировать.
#[cfg(feature = "azalea-buf")]
fn read_nbt_root_safe(buf: &mut Cursor<&[u8]>) -> Result<Nbt, BufReadError> {
    // Читаем первый байт — тип корневого NBT-тега
    let mut type_byte = [0u8; 1];
    buf.read_exact(&mut type_byte).map_err(io_err)?;
    let root_type = type_byte[0];

    match root_type {
        // Compound (10) — стандартный случай, читаем полноценно
        10 => {
            let compound = simdnbt::borrow::read_compound(buf)
                .map_err(|e| BufReadError::Custom(format!("NBT compound read error: {e}")))?;
            // В simdnbt 0.10 BaseNbtCompound -> owned::NbtCompound через to_owned()
            let owned: NbtCompound = compound.to_owned();
            Ok(Nbt::Compound(owned))
        }

        // Short (2)
        2 => {
            let mut skip = [0u8; 2];
            buf.read_exact(&mut skip).map_err(io_err)?;
            Ok(empty_nbt())
        }

        // Byte (1)
        1 => {
            let mut skip = [0u8; 1];
            buf.read_exact(&mut skip).map_err(io_err)?;
            Ok(empty_nbt())
        }

        // Int (3)
        3 => {
            let mut skip = [0u8; 4];
            buf.read_exact(&mut skip).map_err(io_err)?;
            Ok(empty_nbt())
        }

        // Long (4)
        4 => {
            let mut skip = [0u8; 8];
            buf.read_exact(&mut skip).map_err(io_err)?;
            Ok(empty_nbt())
        }

        // Float (5)
        5 => {
            let mut skip = [0u8; 4];
            buf.read_exact(&mut skip).map_err(io_err)?;
            Ok(empty_nbt())
        }

        // Double (6)
        6 => {
            let mut skip = [0u8; 8];
            buf.read_exact(&mut skip).map_err(io_err)?;
            Ok(empty_nbt())
        }

        // ByteArray (7) — i32 длина + N байт
        7 => {
            let len = read_i32(buf)?;
            if len > 0 {
                let mut skip = vec![0u8; len as usize];
                buf.read_exact(&mut skip).map_err(io_err)?;
            }
            Ok(empty_nbt())
        }

        // String (8) — VarInt-длина + UTF-8 байты
        8 => {
            let _s: String = String::azalea_read(buf)?;
            Ok(empty_nbt())
        }

        // List (9) — тип элемента + i32 длина + элементы
        9 => {
            let _inner_type = {
                let mut b = [0u8; 1];
                buf.read_exact(&mut b).map_err(io_err)?;
                b[0]
            };
            let _len = read_i32(buf)?;
            // Не читаем содержимое — см. примечание в тексте ниже.
            Ok(empty_nbt())
        }

        // IntArray (11) — i32 длина + N * 4 байт
        11 => {
            let len = read_i32(buf)?;
            if len > 0 {
                let mut skip = vec![0u8; len as usize * 4];
                buf.read_exact(&mut skip).map_err(io_err)?;
            }
            Ok(empty_nbt())
        }

        // LongArray (12) — i32 длина + N * 8 байт
        12 => {
            let len = read_i32(buf)?;
            if len > 0 {
                let mut skip = vec![0u8; len as usize * 8];
                buf.read_exact(&mut skip).map_err(io_err)?;
            }
            Ok(empty_nbt())
        }

        other => {
            eprintln!("[NBT] Unknown root tag type: {other}, returning empty compound");
            Ok(empty_nbt())
        }
    }
}

#[cfg(feature = "azalea-buf")]
fn read_i32(buf: &mut Cursor<&[u8]>) -> Result<i32, BufReadError> {
    let mut b = [0u8; 4];
    buf.read_exact(&mut b).map_err(io_err)?;
    Ok(i32::from_be_bytes(b))
}

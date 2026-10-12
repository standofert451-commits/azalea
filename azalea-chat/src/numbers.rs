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
                // `Nbt::write` принимает только `&mut Vec<u8>`,
                // поэтому собираем во временный буфер и пишем в `buf`.
                let mut tmp = Vec::new();
                style.write(&mut tmp);
                buf.write_all(&tmp)?;
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

/// Читает NBT-тег из буфера. Корректно обрабатывает Compound (сетевой NBT
/// без имени) и пропускает данные для остальных типов корневых тегов
/// (Byte, Short, Int, Long, Float, Double, ByteArray, String, List,
/// IntArray, LongArray).
#[cfg(feature = "azalea-buf")]
fn read_nbt_root_safe(buf: &mut Cursor<&[u8]>) -> Result<Nbt, BufReadError> {
    let start = buf.position();
    let mut type_byte = [0u8; 1];
    buf.read_exact(&mut type_byte).map_err(io_err)?;
    let root_type = type_byte[0];

    // Откатываемся на 1 байт, чтобы `read_unnamed` прочитал всё сам.
    buf.set_position(start);

    match root_type {
        // Compound (10) — сетевой NBT без имени
        10 => simdnbt::owned::read_unnamed(buf)
            .map_err(|e| BufReadError::Custom(format!("NBT read error: {e}"))),

        // END (0) — пусто
        0 => Ok(Nbt::None),

        // Short (2)
        2 => {
            buf.set_position(buf.position() + 2);
            Ok(Nbt::None)
        }
        // Byte (1)
        1 => {
            buf.set_position(buf.position() + 1);
            Ok(Nbt::None)
        }
        // Int (3)
        3 => {
            buf.set_position(buf.position() + 4);
            Ok(Nbt::None)
        }
        // Long (4)
        4 => {
            buf.set_position(buf.position() + 8);
            Ok(Nbt::None)
        }
        // Float (5)
        5 => {
            buf.set_position(buf.position() + 4);
            Ok(Nbt::None)
        }
        // Double (6)
        6 => {
            buf.set_position(buf.position() + 8);
            Ok(Nbt::None)
        }
        // ByteArray (7): i32-длина (байты) + сами байты
        7 => {
            buf.set_position(buf.position() + 1); // пропустить сам тег-байт
            let len = read_i32(buf)?;
            if len > 0 {
                buf.set_position(buf.position() + len as u64);
            }
            Ok(Nbt::None)
        }
        // String (8): VarInt длина + UTF-8 байты
        8 => {
            buf.set_position(buf.position() + 1);
            let _s: String = String::azalea_read(buf)?;
            Ok(Nbt::None)
        }
        // List (9): тип элемента (1 байт) + i32 длина + элементы
        // Мы не читаем элементы — предполагаем, что для number_format
        // сервер шлёт Compound или Short. Если вдруг прилетит List —
        // это будет ошибкой синхронизации, но так пакет хотя бы
        // не свалится с паникой.
        9 => {
            buf.set_position(buf.position() + 1); // тег-байт
            let mut inner_type = [0u8; 1];
            buf.read_exact(&mut inner_type).map_err(io_err)?;
            let _len = read_i32(buf)?;
            Ok(Nbt::None)
        }
        // IntArray (11): i32 длина (в int) + i32 * N
        11 => {
            buf.set_position(buf.position() + 1);
            let len = read_i32(buf)?;
            if len > 0 {
                buf.set_position(buf.position() + (len as u64) * 4);
            }
            Ok(Nbt::None)
        }
        // LongArray (12): i32 длина (в long) + i64 * N
        12 => {
            buf.set_position(buf.position() + 1);
            let len = read_i32(buf)?;
            if len > 0 {
                buf.set_position(buf.position() + (len as u64) * 8);
            }
            Ok(Nbt::None)
        }

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

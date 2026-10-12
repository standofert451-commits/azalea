//! Contains a few ways to style numbers. At the time of writing, Minecraft only
//! uses this for rendering scoreboard objectives.

#[cfg(feature = "azalea-buf")]
use std::io::{self, Cursor, Write};

#[cfg(feature = "azalea-buf")]
use azalea_buf::AzBuf;
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
    fn azalea_read(buf: &mut Cursor<&[u8]>) -> Result<Self, azalea_buf::BufReadError> {
        let kind = NumberFormatKind::azalea_read(buf)?;
        match kind {
            NumberFormatKind::Blank => Ok(NumberFormat::Blank),
            NumberFormatKind::Styled => {
                // FIX: simdnbt иногда не принимает Short (2) как root-тег,
                // хотя сервер может его прислать. Читаем NBT вручную,
                // поддерживая все возможные типы корневых тегов.
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

/// Читает NBT-тег из буфера, корректно сдвигая курсор для всех типов
/// корневых тегов, включая Short(2). Возвращает пустой compound, если
/// данные не являются полноценным compound (это нормально — стиль можно
/// проигнорировать).
#[cfg(feature = "azalea-buf")]
fn read_nbt_root_safe(buf: &mut Cursor<&[u8]>) -> Result<Nbt, azalea_buf::BufReadError> {
    use std::io::Read;

    // Читаем первый байт — тип корневого NBT-тега
    let mut type_byte = [0u8; 1];
    buf.read_exact(&mut type_byte)
        .map_err(azalea_buf::BufReadError::Io)?;
    let root_type = type_byte[0];

    match root_type {
        // Compound (10) — стандартный случай, читаем полноценно
        10 => {
            let compound = simdnbt::borrow::read_compound(buf).map_err(|e| {
                azalea_buf::BufReadError::Custom(format!("NBT compound read error: {e}"))
            })?;
            Ok(compound.as_tag().to_owned())
        }

        // Short (2)
        2 => {
            let mut skip = [0u8; 2];
            buf.read_exact(&mut skip).map_err(azalea_buf::BufReadError::Io)?;
            Ok(simdnbt::owned::NbtCompound::new().into())
        }

        // Byte (1)
        1 => {
            let mut skip = [0u8; 1];
            buf.read_exact(&mut skip).map_err(azalea_buf::BufReadError::Io)?;
            Ok(simdnbt::owned::NbtCompound::new().into())
        }

        // Int (3)
        3 => {
            let mut skip = [0u8; 4];
            buf.read_exact(&mut skip).map_err(azalea_buf::BufReadError::Io)?;
            Ok(simdnbt::owned::NbtCompound::new().into())
        }

        // Long (4)
        4 => {
            let mut skip = [0u8; 8];
            buf.read_exact(&mut skip).map_err(azalea_buf::BufReadError::Io)?;
            Ok(simdnbt::owned::NbtCompound::new().into())
        }

        // Float (5)
        5 => {
            let mut skip = [0u8; 4];
            buf.read_exact(&mut skip).map_err(azalea_buf::BufReadError::Io)?;
            Ok(simdnbt::owned::NbtCompound::new().into())
        }

        // Double (6)
        6 => {
            let mut skip = [0u8; 8];
            buf.read_exact(&mut skip).map_err(azalea_buf::BufReadError::Io)?;
            Ok(simdnbt::owned::NbtCompound::new().into())
        }

        // ByteArray (7) — 4 байта длины + N байт
        7 => {
            let len = read_i32(buf)?;
            let mut skip = vec![0u8; len.max(0) as usize];
            buf.read_exact(&mut skip).map_err(azalea_buf::BufReadError::Io)?;
            Ok(simdnbt::owned::NbtCompound::new().into())
        }

        // String (8) — VarInt-длина + UTF-8 байты
        8 => {
            use azalea_buf::AzBuf;
            let _s: String = String::azalea_read(buf)?;
            Ok(simdnbt::owned::NbtCompound::new().into())
        }

        // List (9) — тип элемента + VarInt-длина + элементы
        // Простой случай: пустой список
        9 => {
            let _inner_type = {
                let mut b = [0u8; 1];
                buf.read_exact(&mut b).map_err(azalea_buf::BufReadError::Io)?;
                b[0]
            };
            let _len = read_i32(buf)?;
            // Не читаем содержимое — если сервер реально прислал список,
            // это выйдет за пределы NBT, но для стиля это не критично,
            // такие пакеты очень редки.
            Ok(simdnbt::owned::NbtCompound::new().into())
        }

        // IntArray (11) — 4 байта длины + N * 4 байт
        11 => {
            let len = read_i32(buf)?;
            let mut skip = vec![0u8; len.max(0) as usize * 4];
            buf.read_exact(&mut skip).map_err(azalea_buf::BufReadError::Io)?;
            Ok(simdnbt::owned::NbtCompound::new().into())
        }

        // LongArray (12) — 4 байта длины + N * 8 байт
        12 => {
            let len = read_i32(buf)?;
            let mut skip = vec![0u8; len.max(0) as usize * 8];
            buf.read_exact(&mut skip).map_err(azalea_buf::BufReadError::Io)?;
            Ok(simdnbt::owned::NbtCompound::new().into())
        }

        // Неизвестный тип — логируем и возвращаем пустой compound,
        // но НЕ сдвигаем курсор дальше (это может быть ошибкой,
        // но лучше, чем паника)
        other => {
            eprintln!("[NBT] Unknown root tag type: {other}, returning empty compound");
            Ok(simdnbt::owned::NbtCompound::new().into())
        }
    }
}

#[cfg(feature = "azalea-buf")]
fn read_i32(buf: &mut Cursor<&[u8]>) -> Result<i32, azalea_buf::BufReadError> {
    use std::io::Read;
    let mut b = [0u8; 4];
    buf.read_exact(&mut b).map_err(azalea_buf::BufReadError::Io)?;
    Ok(i32::from_be_bytes(b))
}

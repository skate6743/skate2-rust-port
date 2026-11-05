//! EA Localizer formatting as the frontend uses it: `FormatStringCustom`
//! (`0x8297fee8`, with `mAllArgumentsStrings`), `FormatStringExplicit`
//! (`0x8297f690`), `FormatNumber` (`0x82980468`, `0x82980a80`) and the
//! `FEBackEnd` custom argument types (`FEBackEnd::Init` `0x82570a40`).
//! `LanguageManager::Initialize` registers the `#` format prefix
//! (`0x82d66b08`); results here omit it, as `SkateAptString` does.
use super::assets::Assets;
use crate::gpu_renderer::RenderResult;
use skate2_core::challenge::system::MissionTitle;
use skate2_formats::localization::string_hash;

/// `ProfileBackEnd::tSettings::Default` (`0x825644d8`): `eUnitsType_Imperial`,
/// which `ProcessDistance` and `ProcessSpeed` test.
const IMPERIAL: bool = true;

/// `GetPackedString(label, true)` (`0x8297f2f0`): a missing label gives the
/// manager's default string, the single space `LanguageManager` sets.
pub(super) fn packed_string(assets: &Assets, label: &[u8]) -> Vec<u16> {
    packed_hash(assets, string_hash(label))
}

/// `GetPackedString` for a label's hash.
fn packed_hash(assets: &Assets, label: u32) -> Vec<u16> {
    assets
        .localized
        .get(&label)
        .cloned()
        .or_else(|| super::assets::skate3_label(label))
        .unwrap_or_else(|| vec![u16::from(b' ')])
}

/// `mLanguageDetails.mNumberFormatting` from `SetupNumberFormat`
/// (`0x829808e0`): the pattern's first separator groups digits, at its index;
/// a later one is the decimal point. Unset characters are `-`.
struct NumberFormat {
    group: u16,
    group_length: i32,
    decimal: u16,
}

fn number_format(assets: &Assets) -> NumberFormat {
    let fallback: Vec<u16> = "XXX,XXX.XX".encode_utf16().collect();
    let pattern = assets
        .localized
        .get(&string_hash(b"ID_COMMON_NUMBER_FORMAT"))
        .filter(|p| !p.is_empty())
        .unwrap_or(&fallback);
    let mut format = NumberFormat { group: u16::from(b'-'), group_length: -1, decimal: u16::from(b'-') };
    for (i, &c) in pattern.iter().enumerate() {
        if matches!(c, 0x2e | 0x2c | 0x20 | 0xa0) {
            if format.group_length < 0 {
                format.group_length = i32::try_from(i).unwrap_or(i32::MAX);
            }
            if format.group == u16::from(b'-') {
                format.group = c;
            } else {
                format.decimal = c;
            }
        }
    }
    format
}

/// `FormatNumber(char*, int, bool)` (`0x82980468`) without the prefix.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "native byte store of number % 10 + '0'")]
pub(super) fn format_int(assets: &Assets, mut number: i32) -> String {
    let format = number_format(assets);
    if number == 0 {
        return "0".into();
    }
    let mut reversed = Vec::new();
    let mut count = 0;
    while number != 0 {
        if format.group_length > 0 {
            count += 1;
            if count > format.group_length {
                reversed.push(format.group);
                count = 1;
            }
        }
        reversed.push(u16::from((number % 10 + 48) as u8));
        number /= 10;
    }
    reversed.reverse();
    String::from_utf16_lossy(&reversed)
}

/// `FormatNumber(char*, float, bool, uint)` (`0x82980a80`) without the
/// prefix: `FormatNumberInternal` with the number format's separators.
pub(super) fn format_float(assets: &Assets, number: f32, precision: u32) -> String {
    format_number_internal(number, &number_format(assets), precision)
}

/// `mLanguageDetails.mCurrencyFormatting` from `SetupCurrencyFormat`
/// (`0x82981418`) over `ID_COMMON_CURRENCY_SIMPLE_FORMAT`
/// (`LanguageManager::Initialize` `0x82d66a80`), `$XXX,XXX.XX` when missing:
/// the text before the first `X` is the symbol, put before the digits; a
/// separator after three `X`s groups them and a later one is the decimal
/// point; text after six `X`s (eight with a decimal point) is a symbol put
/// after the digits. Unset characters are `-`; the group length starts at 3.
struct CurrencyFormat {
    symbol: Vec<u16>,
    before: bool,
    number: NumberFormat,
    decimals: bool,
}

fn currency_format(assets: &Assets) -> CurrencyFormat {
    let fallback: Vec<u16> = "$XXX,XXX.XX".encode_utf16().collect();
    let pattern = assets
        .localized
        .get(&string_hash(b"ID_COMMON_CURRENCY_SIMPLE_FORMAT"))
        .filter(|p| !p.is_empty())
        .unwrap_or(&fallback);
    let dash = u16::from(b'-');
    let mut format = CurrencyFormat {
        symbol: Vec::new(),
        before: false,
        number: NumberFormat { group: dash, group_length: 3, decimal: dash },
        decimals: false,
    };
    let mut digits = 0;
    for (i, &c) in pattern.iter().enumerate() {
        if c == u16::from(b'X') {
            if digits == 0 {
                format.before = i != 0;
                if format.before && format.symbol.is_empty() {
                    format.symbol = pattern[..i].to_vec();
                }
            }
            digits += 1;
            continue;
        }
        let point = c == u16::from(b'.') || c == u16::from(b',');
        let separator = if point && (3..8).contains(&digits) {
            true
        } else if digits == 3 && !point {
            c == u16::from(b' ') || c == 0xa0
        } else {
            let symbol_after = (digits == 6 && format.number.decimal == dash) || (digits == 8 && format.number.decimal != dash);
            if symbol_after && format.symbol.is_empty() {
                format.before = false;
                format.symbol = pattern[i..].to_vec();
            }
            false
        };
        if separator {
            if format.number.group == dash {
                format.number.group = c;
                format.number.group_length = i32::try_from(i - format.symbol.len()).unwrap_or(i32::MAX);
            } else if format.number.decimal == dash && point {
                format.number.decimal = c;
                format.decimals = true;
            }
        }
    }
    format
}

/// `FormatCurrency(string, number, false)` (`0x82981318`), as
/// `FELanguage.GetLocalizedFormatCurrency` (`0x82604418`) calls it: the
/// symbol and `FormatNumberInternal` with the currency's separators and
/// two decimals when the format has a decimal point.
pub(super) fn format_currency(assets: &Assets, number: f32) -> String {
    let format = currency_format(assets);
    let digits = format_number_internal(number, &format.number, if format.decimals { 2 } else { 0 });
    let symbol = String::from_utf16_lossy(&format.symbol);
    if format.before { format!("{symbol}{digits}") } else { format!("{digits}{symbol}") }
}

/// `FormatNumberInternal` (`0x82980610`) without the prefix: digits come
/// from the double value divided by descending powers of ten (`fnmsub`
/// remainder), then `precision` decimals, then the remainder rounds the
/// last digit half up, carrying over nines, the decimal point and groups.
#[expect(clippy::cast_possible_truncation, reason = "native fctiwz and byte stores")]
#[expect(clippy::cast_sign_loss, reason = "native byte stores")]
fn format_number_internal(number: f32, format: &NumberFormat, precision: u32) -> String {
    let (group, decimal) = (format.group, format.decimal);
    let grouping = format.group_length > 0;
    let mut buffer: Vec<u16> = vec![u16::from(b'0')];
    let mut remainder = f64::from(number);
    let mut power = 1.0_f64;
    let mut exponent = 0_i32;
    if remainder > 1.0 {
        loop {
            power *= 10.0;
            exponent += 1;
            if power.partial_cmp(&remainder) != Some(std::cmp::Ordering::Less) {
                break;
            }
        }
    }
    if power > remainder {
        power *= 0.1;
        exponent -= 1;
    }
    let digits = exponent + 1;
    let mut position = digits;
    if power >= 1.0 {
        loop {
            if grouping {
                if position % format.group_length == 0 && position != digits {
                    buffer.push(group);
                }
                position -= 1;
            }
            let digit = f64::from((remainder / power) as i32 as i8);
            buffer.push(u16::from((digit as i32 + 48) as u8));
            remainder = (-digit).mul_add(power, remainder);
            power *= 0.1;
            if power < 1.0 {
                break;
            }
        }
        if buffer.len() == 1 {
            buffer.push(u16::from(b'0'));
        }
    } else {
        buffer.push(u16::from(b'0'));
    }
    if precision != 0 {
        buffer.push(decimal);
        for _ in 0..precision {
            let scaled = remainder * 10.0;
            let digit = (scaled as i32) as i8;
            buffer.push(u16::from((i32::from(digit) + 48) as u8));
            remainder = scaled - f64::from(digit);
        }
    }
    let mut start = 1;
    if remainder * 10.0 >= 5.0 {
        let mut at = buffer.len() - 1;
        loop {
            let c = buffer[at];
            if c == u16::from(b'9') {
                buffer[at] = u16::from(b'0');
            } else if !(c == decimal || (grouping && c == group)) {
                break;
            }
            at -= 1;
        }
        buffer[at] += 1;
        if at < 1 {
            start = 0;
        }
    }
    String::from_utf16_lossy(&buffer[start..])
}

/// `rw::core::stdc::ConvertAToI` (`0x82bc3ce0`).
fn convert_atoi(text: &[u8]) -> i32 {
    let mut at = text.iter().position(|&c| c == b'-' || c.is_ascii_digit()).unwrap_or(text.len());
    let negative = text.get(at) == Some(&b'-');
    if negative {
        at += 1;
    }
    let mut value = 0_i32;
    while let Some(&c) = text.get(at).filter(|c| c.is_ascii_digit()) {
        value = value.wrapping_mul(10).wrapping_add(i32::from(c - b'0'));
        at += 1;
    }
    if negative { value.wrapping_neg() } else { value }
}

/// `rw::core::stdc::ConvertAToF` (`0x82bc3e68`): single-precision digits
/// (`fmadds`) divided once by the power of ten after the point, then the
/// exponent applied in steps of 10^4, 10^8 and 10.
fn convert_atof(text: &[u8]) -> f32 {
    let mut at = text.iter().position(|&c| c == b'-' || c == b'.' || c.is_ascii_digit()).unwrap_or(text.len());
    let (mut negative, mut point) = (false, false);
    let (mut digits, mut scale) = (0.0_f32, 1.0_f32);
    while let Some(&c) = text.get(at) {
        match c {
            b'0'..=b'9' => {
                digits = digits.mul_add(10.0, f32::from(c - b'0'));
                if point {
                    scale *= 10.0;
                }
            }
            b'.' => point = true,
            b'-' => negative = true,
            _ => break,
        }
        at += 1;
    }
    let mut value = digits / scale;
    if negative {
        value = -value;
    }
    if !matches!(text.get(at), Some(b'E' | b'e')) {
        return value;
    }
    let (mut exponent, mut negative_exponent) = (0_i32, false);
    for &c in text[at + 1..].iter().take_while(|c| c.is_ascii_digit() || **c == b'-') {
        if c == b'-' {
            negative_exponent = true;
        } else {
            exponent = 10 * exponent + i32::from(c - b'0');
        }
    }
    if negative_exponent {
        exponent = -exponent;
    }
    let tenth = f32::from_bits(0x3dcc_cccd);
    if exponent < 0 {
        if exponent <= -4 {
            let steps = (-4 - exponent) / 4 + 1;
            exponent += 4 * steps;
            for _ in 0..steps {
                value *= f32::from_bits(0x38d1_b717);
            }
        }
        if -exponent >= 8 {
            let steps = (-8 - exponent) / 8 + 1;
            exponent += 8 * steps;
            for _ in 0..steps * 8 {
                value *= tenth;
            }
        }
        for _ in 0..-exponent {
            value *= tenth;
        }
    } else if exponent > 0 {
        if exponent >= 4 {
            for _ in 0..exponent / 4 {
                value *= 10000.0;
            }
            exponent &= 3;
        }
        for _ in 0..exponent {
            value *= 10.0;
        }
    }
    value
}

/// The `FEBackEnd` custom argument types the frontend registers. Each
/// callback writes at most 63 characters (`Snprintf` into 64).
fn custom_argument(assets: &Assets, name: &[u16], value: &[u8]) -> RenderResult<Vec<u16>> {
    let unit = |label: &[u8]| String::from_utf16_lossy(&packed_string(assets, label));
    let text = match String::from_utf16_lossy(name).as_str() {
        // ProcessPlayerName 0x8255a4d0, ProfileBackEnd::mProfileData.mProfileName.
        "PLAYER" => assets.profile_name.clone(),
        "PROFILMPRO"=>assets.pro_film_pro_name.clone(),
        // ProcessPoolFound 0x82563320, ProcessPoolAboutToBeFound 0x825633e0 and ProcessTapesGiven
        // 0x82563960: %d of a points pool.
        "POOL_FOUND" => assets.frontend_arguments.pools_found.to_string(),
        "POOLTOBEFOUND" => (assets.frontend_arguments.pools_found + 1).to_string(),
        "TAPEGIVEN" => assets.frontend_arguments.tapes_given.to_string(),
        // ProcessSponsorWorld 0x825634a0 and ProcessDemoWorld 0x82563610: %s of the world.
        "SponsorWorld" => assets.frontend_arguments.sponsor_world.clone(),
        "DemoWorld" => assets.frontend_arguments.demo_world.clone(),
        // ProcessDemoSponsor 0x82563780: GetBoardSponsorHALID's packed string.
        "DEMOSPONSOR" => unit(assets.frontend_arguments.demo_sponsor.as_bytes()),
        // Process{Board,Truck,Wheel,Shoe}Sponsor 0x82562e80..0x82563320.
        kind @ ("BOARD" | "TRUCK" | "WHEEL" | "SHOE") => {
            let index = match kind { "BOARD" => 0, "TRUCK" => 1, "WHEEL" => 2, _ => 3 };
            unit(assets.sponsor_labels[index].as_bytes())
        }
        // ProcessPoints 0x8255a468.
        "points" => format_int(assets, convert_atoi(value)),
        // ProcessDistance 0x8255a2b8: feet are metres * 3.
        "distance" if IMPERIAL => format!("{} {}", format_float(assets, convert_atof(value) * 3.0, 1), unit(b"ID_COMMON_METRIC_NEW_FEET")),
        "distance" => format!("{} {}", format_float(assets, convert_atof(value), 1), unit(b"ID_COMMON_METRIC_DISTANCE")),
        // ProcessTime 0x8255a358.
        "time" => format!("{} {}", format_float(assets, convert_atof(value), 1), unit(b"ID_COMMON_METRIC_TIME")),
        // ProcessSpeed 0x8255a3b8.
        "speed" if IMPERIAL => format!("{} {}", format_float(assets, convert_atof(value) * 2.25, 1), unit(b"ID_COMMON_MPH")),
        "speed" => format!("{} {}", format_float(assets, convert_atof(value) * f32::from_bits(0x4066_6666), 1), unit(b"ID_COMMON_KMPH")),
        other => return Err(format!("untranslated Localizer argument type {other}").into()),
    };
    Ok(text.encode_utf16().take(63).collect())
}

/// The argument types `LocalizerManager::Initialize` (`0x82980058`) and
/// `FEBackEnd::Init` register; every other name asserts and reads as text.
const CUSTOM_TYPES: [&str; 16] = [
    "distance", "time", "speed", "points", "PLAYER", "BOARD", "TRUCK", "WHEEL", "SHOE", "POOL_FOUND",
    "POOLTOBEFOUND", "SponsorWorld", "DemoWorld", "DEMOSPONSOR", "PROFILMPRO", "TAPEGIVEN",
];

/// `FormatStringCustom`'s argument split: commas separate up to twenty
/// arguments of at most 63 characters (a longer one is cut and the next
/// character skipped).
fn split_arguments(list: &[u8]) -> Vec<Vec<u8>> {
    let mut arguments = vec![Vec::new(); 20];
    let (mut index, mut at) = (0, 0);
    while index < 20 && at < list.len() {
        if list[at] != b',' {
            arguments[index].push(list[at]);
            at += 1;
            if at < list.len() && list[at] != b',' && arguments[index].len() < 64 {
                continue;
            }
        }
        if arguments[index].len() >= 64 {
            arguments[index].truncate(63);
        }
        index += 1;
        if at < list.len() {
            at += 1;
        }
    }
    arguments
}

#[derive(Clone, PartialEq)]
enum ArgumentType {
    Text,
    Custom(Vec<u16>),
    /// `d` (`eFORMAT_ARG_INT`, 1) of a typed `FormatString`: `FormatNumber(int, 0)`.
    Integer,
    /// `f` (`eFORMAT_ARG_FLOAT`, 2) of a typed `FormatString`: the double argument rounded to
    /// single (`frsp`) through `FormatNumber(float, false, mPrecision)`.
    Float,
}

struct Token {
    start: usize,
    length: usize,
    index: i32,
    kind: ArgumentType,
    /// `mPrecision`: the digit after the token's `.`.
    precision: Option<u32>,
}

/// `FormatStringExplicit` (`0x8297f690`): `{n:type}` tokens (a `{{` is not one)
/// take argument slots in index order; an index without a token takes no slot.
/// `FormatStringCustom` reads every non-custom type as text
/// (`mAllArgumentsStrings`); a typed `FormatString` (`typed`) shows a `d` token's
/// integer and an `f` token's float through `FormatNumber` and copies an `s` token's string
/// (`Initialize` `0x82980058` registers `s` as `eFORMAT_ARG_CHAR`).
fn format_explicit(assets: &Assets, format: &[u16], arguments: &[Vec<u8>], typed: bool) -> RenderResult<Vec<u16>> {
    let (brace, close, colon, dot) = (u16::from(b'{'), u16::from(b'}'), u16::from(b':'), u16::from(b'.'));
    let mut tokens: Vec<Token> = Vec::new();
    let (mut open, mut start, mut index, mut kind, mut highest) = (false, 0, -1, ArgumentType::Text, -1);
    let mut precision = None;
    let mut at = 0;
    while at < format.len() {
        if format[at] == brace && format.get(at + 1) != Some(&brace) {
            open = true;
            start = at;
            precision = None;
        }
        if open {
            if format[at] == colon {
                index = i32::from(format[at - 1]) - 48;
                highest = highest.max(index);
                at += 1;
                let mut name = Vec::new();
                while let Some(&c) = format.get(at) {
                    if c == close || name.len() >= 16 || c == 0 {
                        break;
                    }
                    if c == dot {
                        at += 1;
                        precision = Some(precision_digit(format, at));
                    } else {
                        name.push(c);
                    }
                    at += 1;
                }
                kind = if CUSTOM_TYPES.iter().any(|t| t.encode_utf16().eq(name.iter().copied())) {
                    ArgumentType::Custom(name)
                } else if typed && name == [u16::from(b'd')] {
                    ArgumentType::Integer
                } else if typed && name == [u16::from(b's')] {
                    ArgumentType::Text
                } else if typed && name == [u16::from(b'f')] {
                    ArgumentType::Float
                } else if typed {
                    return Err(format!("untranslated typed Localizer argument {}", String::from_utf16_lossy(&name)).into());
                } else {
                    ArgumentType::Text
                };
            }
            if format.get(at) == Some(&dot) {
                precision = Some(precision_digit(format, at + 1));
                at += 2;
            }
            if format.get(at) == Some(&close) {
                open = false;
                tokens.push(Token { start, length: at - start + 1, index, kind: kind.clone(), precision });
            }
        }
        at += 1;
    }
    let mut values = vec![Vec::new(); tokens.len()];
    let mut slot = 0;
    for index in 0..=highest {
        let Some(first) = tokens.iter().position(|t| t.index == index) else { continue };
        let kind = tokens[first].kind.clone();
        let argument = arguments.get(slot).map_or(&[][..], Vec::as_slice);
        slot += 1;
        let value = match &kind {
            ArgumentType::Custom(name) => custom_argument(assets, name, argument)?,
            ArgumentType::Integer => format_int(assets, convert_atoi(argument)).encode_utf16().collect(),
            ArgumentType::Float | ArgumentType::Text => argument.iter().map(|&b| u16::from(b)).collect(),
        };
        for (token, slot_value) in tokens.iter().zip(values.iter_mut()) {
            if token.index == index && token.kind == kind {
                if kind == ArgumentType::Float {
                    *slot_value = float_argument(assets, argument, token.precision)?;
                } else {
                    slot_value.clone_from(&value);
                }
            }
        }
    }
    let mut out = Vec::new();
    let (mut at, mut next) = (0, 0);
    while at < format.len() {
        if tokens.get(next).is_some_and(|t| t.start == at) {
            out.extend_from_slice(&values[next]);
            at += tokens[next].length;
            next += 1;
        } else {
            out.push(format[at]);
            at += 1;
        }
    }
    Ok(out)
}

/// A token's `mPrecision`: its digit's code less `'0'` (the terminator's past the end).
fn precision_digit(format: &[u16], at: usize) -> u32 {
    u32::from(format.get(at).copied().unwrap_or(0)).wrapping_sub(48)
}

/// An `f` token's value: the callers pass the float's shortest text, which parses back to it.
/// Without a `.` the token keeps the `mPrecision` an earlier format left in its slot.
fn float_argument(assets: &Assets, argument: &[u8], precision: Option<u32>) -> RenderResult<Vec<u16>> {
    let precision = precision.ok_or("typed Localizer float without a precision of its own")?;
    let number: f32 = std::str::from_utf8(argument)?.parse()?;
    Ok(format_float(assets, number, precision).encode_utf16().collect())
}

/// `FormatStringCustom(StringHash(label), arguments)`, the localized text a
/// `#`-prefixed result shows.
pub(super) fn format_custom(assets: &Assets, label: &str, arguments: &str) -> RenderResult<Vec<u16>> {
    format_explicit(assets, &packed_string(assets, label.as_bytes()), &split_arguments(arguments.as_bytes()), false)
}

/// `Mission.GetTitle()` (`0x8260e208`) as a text field shows it: the title through
/// `FormatStringCustom`, the missing-title placeholder, or the lobby label.
pub(super) fn mission_title(assets: &Assets, title: &MissionTitle) -> RenderResult<Vec<u16>> {
    Ok(match title {
        MissionTitle::Formatted(key) => format_custom(assets, key, "")?,
        MissionTitle::Missing => "Missing Instance Title...".encode_utf16().collect(),
        MissionTitle::Lobby => assets.initial_text("ID_FREESKATE_CHALLENGE_LOBBY"),
        MissionTitle::FreeskateLobby => assets.initial_text("ID_FREESKATE_LOBBY"),
    })
}

/// An argument a script passes `FELanguage.GetLocalizedFormatString`.
pub(super) enum AptArgument<'a> {
    Integer(i32),
    Text(&'a str),
}

/// `FELanguage.GetLocalizedFormatString(label, count, ...)` (`LanguageCmdTranslator::
/// sMethod_GetLocalizedFormatString` `0x82622c00`): an integer argument becomes
/// `FormatNumber(value, false)`, a string stays as it is, and the label formats every
/// argument as a string (`mAllArgumentsStrings`).
pub(super) fn localized_format_string(assets: &Assets, label: &str, arguments: &[AptArgument]) -> RenderResult<Vec<u16>> {
    let arguments: Vec<Vec<u8>> = arguments
        .iter()
        .map(|argument| match argument {
            AptArgument::Integer(value) => format_int(assets, *value).into_bytes(),
            AptArgument::Text(text) => text.as_bytes().to_vec(),
        })
        .collect();
    format_explicit(assets, &packed_string(assets, label.as_bytes()), &arguments, false)
}

/// `FormatString` with the string parameters supplied by CentralHUD (0x826bbf60).
pub(super) fn format_string_text(assets: &Assets, label: &str, arguments: &[String]) -> RenderResult<Vec<u16>> {
    let arguments: Vec<Vec<u8>> = arguments.iter().map(|value| value.as_bytes().to_vec()).collect();
    format_explicit(assets, &packed_string(assets, label.as_bytes()), &arguments, true)
}

/// `FormatString(string, label, value)` (`0x8297fe70`) with one integer: the
/// label's typed tokens through `FormatStringExplicit`, so `ID_COMMON_NUMBER`'s
/// `{0:d}` shows `FormatNumber(value, 0)`.
pub(super) fn format_string_int(assets: &Assets, label: &str, value: i32) -> RenderResult<Vec<u16>> {
    format_explicit(assets, &packed_string(assets, label.as_bytes()), &[value.to_string().into_bytes()], true)
}

/// `FormatString(string, label, value)` (`0x8297fe70`) with one float, a double in the
/// arguments: the label's typed tokens through `FormatStringExplicit`, so
/// `ID_LOGBOOK_STATS_PERCENT_INT`'s `{0:f.0}` shows `FormatNumber(value, false, 0)`.
pub(super) fn format_string_float(assets: &Assets, label: &str, value: f32) -> RenderResult<Vec<u16>> {
    format_explicit(assets, &packed_string(assets, label.as_bytes()), &[value.to_string().into_bytes()], true)
}

/// `FormatString(string, label, ...)` (`0x8297fe70`) with the packed strings
/// (`GetPackedString(.., true)`) of `labels` as its text arguments, as
/// `FrontEndState_Receipt::GetItemText` (`0x82681198`) formats a receipt line.
pub(super) fn format_string_labels(assets: &Assets, label: &str, labels: &[&str]) -> RenderResult<Vec<u16>> {
    let arguments: Vec<Vec<u8>> = labels
        .iter()
        .map(|label| packed_string(assets, label.as_bytes()).into_iter().map(|unit| u8::try_from(unit).unwrap_or(b'?')).collect())
        .collect();
    format_explicit(assets, &packed_string(assets, label.as_bytes()), &arguments, true)
}

/// `FormatString(string, label)` (`0x8297fe70`) with no arguments: the
/// label's packed string through `FormatStringExplicit`.
pub(super) fn format_hal(assets: &Assets, label: u32) -> RenderResult<Vec<u16>> {
    format_explicit(assets, &packed_hash(assets, label), &[], false)
}

/// `mLanguageDetails.mTimeFormatting` (`TimeFormatInfo`) as `SetupTimeFormat` (`0x829811f0`)
/// builds it: `ConstructTimeFormatSpec` (`0x82980cb8`) over `ID_COMMON_TIME_FORMAT`, or
/// `hh:mm:ss.ff` when that is missing or empty. `GetNextToken` (`0x82980aa0`) reads a run of
/// `h`, `m`, `s` or `f` as that field's digit count and `+` as always showing the sign; any other
/// character is the separator after the field before it.
struct TimeFormat {
    hour_digits: usize,
    min_digits: usize,
    sec_digits: usize,
    frac_digits: usize,
    always_show_sign: bool,
    hour_min_sep: u16,
    min_sec_sep: u16,
    sec_frac_sep: u16,
}

fn time_format(assets: &Assets) -> TimeFormat {
    let fallback: Vec<u16> = "hh:mm:ss.ff".encode_utf16().collect();
    let pattern = assets
        .localized
        .get(&string_hash(b"ID_COMMON_TIME_FORMAT"))
        .filter(|p| !p.is_empty())
        .unwrap_or(&fallback);
    let mut format = TimeFormat {
        hour_digits: 0,
        min_digits: 0,
        sec_digits: 0,
        frac_digits: 0,
        always_show_sign: false,
        hour_min_sep: 0,
        min_sec_sep: 0,
        sec_frac_sep: 0,
    };
    // eTIMEFORMAT_ARG_TYPE: hour, minute, second, fraction, negative, sign, none.
    let mut previous = 6;
    let mut at = 0;
    while at < pattern.len() {
        let c = pattern[at];
        let token = match u8::try_from(c) {
            Ok(b'h') => 0,
            Ok(b'm') => 1,
            Ok(b's') => 2,
            Ok(b'f') => 3,
            Ok(b'-') => 4,
            Ok(b'+') => 5,
            _ => 6,
        };
        let mut run = 1;
        if token < 4 {
            while pattern.get(at + run) == Some(&c) {
                run += 1;
            }
        }
        match (token, previous) {
            (0, _) => format.hour_digits = run,
            (1, _) => format.min_digits = run,
            (2, _) => format.sec_digits = run,
            (3, _) => format.frac_digits = run,
            (5, _) => format.always_show_sign = true,
            (6, 0) => format.hour_min_sep = c,
            (6, 1) => format.min_sec_sep = c,
            (6, 2) => format.sec_frac_sep = c,
            _ => {}
        }
        previous = token;
        at += run;
    }
    format
}

/// `CountDigits` (`0x8297e558`): zero has none.
fn count_digits(value: i32) -> usize {
    let mut value = value.unsigned_abs();
    let mut count = 0;
    while value > 0 {
        value /= 10;
        count += 1;
    }
    count
}

/// `IntToString(dest, value, digits, true)` (`0x8297e588`): the lowest `digits` digits with
/// leading zeros.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "native byte store of value % 10 + '0'")]
fn int_digits(out: &mut Vec<u16>, mut value: i32, digits: usize) {
    let start = out.len();
    out.resize(start + digits, 0);
    for at in (start..start + digits).rev() {
        out[at] = u16::from(((value % 10) + 48) as u8);
        value /= 10;
    }
}

/// `DecimalToString(dest, value, digits)` (`0x8297e618`): the value's digits from the most
/// significant (a value below ten gets a leading zero), padded with zeros to `digits`; a longer
/// value is cut and its next digit rounds the last one up.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "native byte stores")]
fn decimal_digits(out: &mut Vec<u16>, value: i32, digits: usize) {
    let mut reversed = Vec::new();
    let mut rest = value;
    while rest != 0 {
        reversed.push(((rest % 10) + 48) as u8);
        rest /= 10;
    }
    if value < 10 {
        reversed.push(b'0');
    }
    let start = out.len();
    for i in 0..digits {
        out.push(u16::from(if i >= reversed.len() { b'0' } else { reversed[reversed.len() - 1 - i] }));
    }
    if digits > 0 && digits < reversed.len() && reversed[reversed.len() - digits - 1] >= b'5' {
        out[start + digits - 1] += 1;
    }
}

/// One character of `BuildTimeFormatString`'s output while it has room.
fn push_time_unit(out: &mut Vec<u16>, room: &mut usize, unit: u16) {
    if *room > 0 {
        out.push(unit);
        *room -= 1;
    }
}

/// One field of `BuildTimeFormatString`: its digits (at least two after a larger field, else at
/// least its own count), then its separator; answers whether room is left.
fn time_field(out: &mut Vec<u16>, room: &mut usize, value: i32, digits: usize, following: bool, separator: u16) -> bool {
    let digits = if following { digits.max(2) } else { digits.max(count_digits(value)) }.min(*room);
    int_digits(out, value, digits);
    *room -= digits;
    if separator != 0 {
        push_time_unit(out, room, separator);
    }
    *room != 0
}

/// `FormatTime(string, hours, minutes, seconds, hundredths, addPrefix, flags)` (`0x829812f0`)
/// through `BuildTimeFormatString` (`0x82980e30`), without the `#` prefix (which takes one of the
/// 29 characters of room). Flag 2 shows the sign (`+`, or `-` with flag 1, which alone shows `-`),
/// 4 shows the hours even at zero and the fraction with them, 16 drops the fraction. A field shows
/// when it is non-zero or a larger one showed (the seconds also for a fraction); the fraction
/// shows only without hours.
pub(super) fn format_time(assets: &Assets, hours: i32, minutes: i32, seconds: i32, hundredths: i32, add_prefix: bool, flags: i32) -> String {
    let format = time_format(assets);
    let mut room: usize = if add_prefix { 28 } else { 29 };
    let mut out: Vec<u16> = Vec::new();
    if format.always_show_sign || flags & 2 != 0 {
        push_time_unit(&mut out, &mut room, u16::from(if flags & 1 == 0 { b'+' } else { b'-' }));
    } else if flags & 1 != 0 {
        push_time_unit(&mut out, &mut room, u16::from(b'-'));
    }
    let mut following = false;
    if format.hour_digits != 0 && (hours != 0 || flags & 4 != 0) {
        following = time_field(&mut out, &mut room, hours, format.hour_digits, false, format.hour_min_sep);
    }
    if format.min_digits != 0 && (minutes != 0 || following) {
        following = time_field(&mut out, &mut room, minutes, format.min_digits, following, format.min_sec_sep);
    }
    if format.sec_digits != 0 && (seconds != 0 || hundredths != 0 || following) {
        following = time_field(&mut out, &mut room, seconds, format.sec_digits, following, 0);
    }
    if format.frac_digits != 0 && (hours == 0 || flags & 4 != 0) && (hundredths != 0 || following) && flags & 0x10 == 0 {
        if format.sec_frac_sep != 0 {
            push_time_unit(&mut out, &mut room, format.sec_frac_sep);
        }
        decimal_digits(&mut out, hundredths, format.frac_digits.min(room));
    }
    String::from_utf16_lossy(&out)
}

#[cfg(test)]
mod tests {
    use super::{convert_atof, convert_atoi, decimal_digits, split_arguments};

    #[test]
    fn native_argument_parsing() {
        let arguments = split_arguments(b"17.7778,2.5");
        assert_eq!((arguments.len(), &arguments[0][..], &arguments[1][..], &arguments[2][..]), (20, &b"17.7778"[..], &b"2.5"[..], &b""[..]));
        assert_eq!(split_arguments(b"a,,b")[..3], [b"a".to_vec(), Vec::new(), b"b".to_vec()]);
        assert_eq!(split_arguments(b" ")[0], b" ");
        assert_eq!(convert_atoi(b"46000"), 46000);
        assert_eq!(convert_atoi(b"x-12y"), -12);
        assert_eq!(convert_atof(b"10.5"), 10.5);
        assert_eq!(convert_atof(b"11.11112"), 11.11112_f32);
        assert_eq!(convert_atof(b""), 0.0);
        assert_eq!(convert_atof(b"2.5e2"), 250.0);
    }

    #[test]
    fn native_decimal_digits() {
        let text = |value: i32, digits: usize| {
            let mut out = Vec::new();
            decimal_digits(&mut out, value, digits);
            String::from_utf16_lossy(&out)
        };
        assert_eq!([text(0, 2), text(5, 2), text(42, 2), text(123, 2), text(167, 2)], ["00", "05", "42", "12", "17"]);
    }
}

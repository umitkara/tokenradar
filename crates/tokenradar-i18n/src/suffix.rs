//! Turkish case suffixes after numbers: `5'te`, `6'da`, `%72'si`, `10:59'da` (architecture §12).
//!
//! The suffix follows the **spoken** form of the number, not its last digit: the last word of
//! its reading decides vowel harmony and consonant hardening. `40` is "kırk", so `40'ta`;
//! `1.000.000` is "bir milyon", so `1.000.000'da`. This is the Rust half of `TR_SUFFIX`; the
//! TypeScript side (Step 09) is checked against the same `testdata/i18n-vectors.json`.

/// A grammatical case (plus the 3rd person possessive) that takes a suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Case {
    /// Locative: `5'te`, `6'da`.
    Loc,
    /// Ablative: `5'ten`, `10'dan`.
    Abl,
    /// Dative: `3'e`, `2'ye`.
    Dat,
    /// Accusative: `3'ü`, `2'yi`.
    Acc,
    /// Genitive: `3'ün`, `2'nin`.
    Gen,
    /// Third person possessive: `%72'si`, `%40'ı`.
    Poss,
}

impl Case {
    /// The Fluent argument (`"loc"`, `"abl"`, …); unknown names are `None`.
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "loc" => Self::Loc,
            "abl" => Self::Abl,
            "dat" => Self::Dat,
            "acc" => Self::Acc,
            "gen" => Self::Gen,
            "poss" => Self::Poss,
            _ => return None,
        })
    }
}

/// `text` followed by an apostrophe and the suffix for its trailing number. Text without a
/// trailing number is returned unchanged.
///
/// The trailing number may be a Turkish-formatted number (`1.234,5`, `%72`) or a time
/// (`10:59`, read by its minutes, or by its hour on the full hour).
pub fn tr_suffix(text: &str, case: Case) -> String {
    let Some(word) = last_word(text) else {
        return text.to_owned();
    };
    format!("{text}'{}", suffix(word, case))
}

/// The last word of the reading of `text`'s trailing number.
fn last_word(text: &str) -> Option<Word> {
    let start = text
        .char_indices()
        .rev()
        .take_while(|&(_, c)| c.is_ascii_digit() || matches!(c, '.' | ',' | ':'))
        .last()
        .map(|(i, _)| i)?;
    let number = text[start..].trim_matches(|c: char| !c.is_ascii_digit());
    if number.is_empty() {
        return None;
    }
    let spoken = if let Some((hour, minute)) = number.rsplit_once(':') {
        // "10:00" is read "on", "10:59" is read "on elli dokuz".
        if minute.bytes().all(|b| b == b'0') {
            hour.rsplit(':').next().unwrap_or(hour)
        } else {
            minute
        }
    } else {
        match number.split_once(',') {
            // "23,5" is read "yirmi üç virgül beş"; "23,0" reads like "23".
            Some((_, frac)) if frac.bytes().any(|b| b != b'0') => frac,
            Some((int, _)) => int,
            None => number,
        }
    };
    let digits: String = spoken.chars().filter(char::is_ascii_digit).collect();
    Some(word_of(&digits))
}

/// The last word of the reading of a digit string (leading zeros allowed).
fn word_of(digits: &str) -> Word {
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return Word::SIFIR;
    }
    let bytes = digits.as_bytes();
    let n = bytes.len();
    let digit = |pos_from_right: usize| bytes[n - 1 - pos_from_right] - b'0';
    if digit(0) != 0 {
        return Word::ONES[usize::from(digit(0))];
    }
    if n > 1 && digit(1) != 0 {
        return Word::TENS[usize::from(digit(1))];
    }
    if n > 2 && digit(2) != 0 {
        return Word::YUZ;
    }
    // The lowest group of three with a non-zero digit names the magnitude.
    let group = (0..n)
        .rev()
        .map(|i| n - 1 - i)
        .find(|&pos| digit(pos) != 0)
        .map_or(0, |pos| pos / 3);
    match group {
        1 => Word::BIN,
        2 => Word::MILYON,
        3 => Word::MILYAR,
        _ => Word::TRILYON,
    }
}

/// What the suffix depends on: the word's last vowel and its final sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Word {
    last_vowel: char,
    ends_in_vowel: bool,
    /// Ends in one of "fıstıkçı şahap" (f s t k ç ş h p): `-te`, `-ten`.
    hard_final: bool,
}

impl Word {
    const fn new(last_vowel: char, ends_in_vowel: bool, hard_final: bool) -> Self {
        Self {
            last_vowel,
            ends_in_vowel,
            hard_final,
        }
    }

    const SIFIR: Self = Self::new('ı', false, false);
    /// sıfır, bir, iki, üç, dört, beş, altı, yedi, sekiz, dokuz.
    const ONES: [Self; 10] = [
        Self::SIFIR,
        Self::new('i', false, false),
        Self::new('i', true, false),
        Self::new('ü', false, true),
        Self::new('ö', false, true),
        Self::new('e', false, true),
        Self::new('ı', true, false),
        Self::new('i', true, false),
        Self::new('i', false, false),
        Self::new('u', false, false),
    ];
    /// (unused), on, yirmi, otuz, kırk, elli, altmış, yetmiş, seksen, doksan.
    const TENS: [Self; 10] = [
        Self::SIFIR,
        Self::new('o', false, false),
        Self::new('i', true, false),
        Self::new('u', false, false),
        Self::new('ı', false, true),
        Self::new('i', true, false),
        Self::new('ı', false, true),
        Self::new('i', false, true),
        Self::new('e', false, false),
        Self::new('a', false, false),
    ];
    const YUZ: Self = Self::new('ü', false, false);
    const BIN: Self = Self::new('i', false, false);
    const MILYON: Self = Self::new('o', false, false);
    const MILYAR: Self = Self::new('a', false, false);
    const TRILYON: Self = Self::new('o', false, false);

    /// Two-way harmony: `a` after back vowels, `e` after front ones.
    fn a_e(self) -> char {
        if matches!(self.last_vowel, 'a' | 'ı' | 'o' | 'u') {
            'a'
        } else {
            'e'
        }
    }

    /// Four-way harmony: `ı i u ü`.
    fn i4(self) -> char {
        match self.last_vowel {
            'a' | 'ı' => 'ı',
            'o' | 'u' => 'u',
            'ö' | 'ü' => 'ü',
            _ => 'i',
        }
    }
}

fn suffix(w: Word, case: Case) -> String {
    let d = if w.hard_final { 't' } else { 'd' };
    let mut s = String::with_capacity(4);
    match case {
        Case::Loc => {
            s.push(d);
            s.push(w.a_e());
        }
        Case::Abl => {
            s.push(d);
            s.push(w.a_e());
            s.push('n');
        }
        Case::Dat => {
            if w.ends_in_vowel {
                s.push('y');
            }
            s.push(w.a_e());
        }
        Case::Acc | Case::Gen | Case::Poss => {
            if w.ends_in_vowel {
                s.push(match case {
                    Case::Acc => 'y',
                    Case::Gen => 'n',
                    _ => 's',
                });
            }
            s.push(w.i4());
            if case == Case::Gen {
                s.push('n');
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffixes_follow_the_spoken_last_word() {
        let cases = [
            ("0", Case::Loc, "0'da"),
            ("12", Case::Loc, "12'de"),
            ("15", Case::Abl, "15'ten"),
            ("101", Case::Dat, "101'e"),
            ("110", Case::Loc, "110'da"),
            ("1.500", Case::Loc, "1.500'de"),
            ("2.000", Case::Abl, "2.000'den"),
            ("1.001.000", Case::Loc, "1.001.000'de"),
            ("3.000.000.000", Case::Loc, "3.000.000.000'da"),
        ];
        for (text, case, out) in cases {
            assert_eq!(tr_suffix(text, case), out, "{text} {case:?}");
        }
    }

    #[test]
    fn decimals_and_times_are_read_like_turkish() {
        assert_eq!(tr_suffix("%23,5", Case::Loc), "%23,5'te");
        assert_eq!(tr_suffix("%23,0", Case::Loc), "%23,0'te");
        assert_eq!(tr_suffix("10:59", Case::Loc), "10:59'da");
        assert_eq!(tr_suffix("Çar 10:00", Case::Loc), "Çar 10:00'da");
        assert_eq!(tr_suffix("10:40", Case::Dat), "10:40'a");
        assert_eq!(tr_suffix("09:05", Case::Dat), "09:05'e");
    }

    #[test]
    fn text_without_a_trailing_number_is_unchanged() {
        assert_eq!(tr_suffix("Çar", Case::Loc), "Çar");
        assert_eq!(tr_suffix("", Case::Loc), "");
        assert_eq!(tr_suffix("72%", Case::Loc), "72%");
    }

    #[test]
    fn case_names_parse() {
        assert_eq!(Case::parse("poss"), Some(Case::Poss));
        assert_eq!(Case::parse("nom"), None);
    }
}

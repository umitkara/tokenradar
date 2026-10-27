// Turkish case suffixes after numbers: `5'te`, `6'da`, `%72'si`, `10:59'da`. A port of
// crates/tokenradar-i18n/src/suffix.rs, checked against the same testdata/i18n-vectors.json.
//
// The suffix follows the spoken form of the number, not its last digit: `40` is "kırk", so
// `40'ta`; `1.000.000` is "bir milyon", so `1.000.000'da`.

export type Case = "loc" | "abl" | "dat" | "acc" | "gen" | "poss";

const CASES: readonly string[] = ["loc", "abl", "dat", "acc", "gen", "poss"];

export function parseCase(s: unknown): Case | null {
  return typeof s === "string" && CASES.includes(s) ? (s as Case) : null;
}

/** What the suffix depends on: the word's last vowel and its final sound. */
type Word = {
  lastVowel: string;
  endsInVowel: boolean;
  /** Ends in one of "fıstıkçı şahap" (f s t k ç ş h p). */
  hardFinal: boolean;
};

const w = (lastVowel: string, endsInVowel: boolean, hardFinal: boolean): Word => ({
  lastVowel,
  endsInVowel,
  hardFinal,
});

const SIFIR = w("ı", false, false);
// sıfır, bir, iki, üç, dört, beş, altı, yedi, sekiz, dokuz
const ONES = [
  SIFIR,
  w("i", false, false),
  w("i", true, false),
  w("ü", false, true),
  w("ö", false, true),
  w("e", false, true),
  w("ı", true, false),
  w("i", true, false),
  w("i", false, false),
  w("u", false, false),
];
// (unused), on, yirmi, otuz, kırk, elli, altmış, yetmiş, seksen, doksan
const TENS = [
  SIFIR,
  w("o", false, false),
  w("i", true, false),
  w("u", false, false),
  w("ı", false, true),
  w("i", true, false),
  w("ı", false, true),
  w("i", false, true),
  w("e", false, false),
  w("a", false, false),
];
const YUZ = w("ü", false, false);
const BIN = w("i", false, false);
const MILYON = w("o", false, false);
const MILYAR = w("a", false, false);
const TRILYON = w("o", false, false);

/** `text` followed by an apostrophe and the suffix for its trailing number, if it has one. */
export function trSuffix(text: string, c: Case): string {
  const word = lastWord(text);
  return word ? `${text}'${suffix(word, c)}` : text;
}

function lastWord(text: string): Word | null {
  const m = /[0-9.,:]+$/.exec(text);
  if (!m) return null;
  const number = m[0].replace(/^[^0-9]+|[^0-9]+$/g, "");
  if (!number) return null;
  let spoken: string;
  const colon = number.lastIndexOf(":");
  if (colon >= 0) {
    // "10:00" is read "on", "10:59" is read "on elli dokuz".
    const hour = number.slice(0, colon);
    const minute = number.slice(colon + 1);
    spoken = /^0*$/.test(minute) ? (hour.split(":").pop() ?? hour) : minute;
  } else {
    const comma = number.indexOf(",");
    if (comma >= 0) {
      // "23,5" is read "yirmi üç virgül beş"; "23,0" reads like "23".
      const frac = number.slice(comma + 1);
      spoken = /[1-9]/.test(frac) ? frac : number.slice(0, comma);
    } else {
      spoken = number;
    }
  }
  return wordOf(spoken.replace(/[^0-9]/g, ""));
}

/** The last word of the reading of a digit string (leading zeros allowed). */
function wordOf(raw: string): Word {
  const digits = raw.replace(/^0+/, "");
  if (!digits) return SIFIR;
  const n = digits.length;
  const digit = (fromRight: number) => Number(digits[n - 1 - fromRight]);
  if (digit(0) !== 0) return ONES[digit(0)] ?? SIFIR;
  if (n > 1 && digit(1) !== 0) return TENS[digit(1)] ?? SIFIR;
  if (n > 2 && digit(2) !== 0) return YUZ;
  // The lowest group of three with a non-zero digit names the magnitude.
  let group = 0;
  for (let pos = 0; pos < n; pos++) {
    if (digit(pos) !== 0) {
      group = Math.floor(pos / 3);
      break;
    }
  }
  return [SIFIR, BIN, MILYON, MILYAR][group] ?? TRILYON;
}

/** Two-way harmony: `a` after back vowels, `e` after front ones. */
const aE = (word: Word) => ("aıou".includes(word.lastVowel) ? "a" : "e");

/** Four-way harmony: `ı i u ü`. */
function i4(word: Word): string {
  if ("aı".includes(word.lastVowel)) return "ı";
  if ("ou".includes(word.lastVowel)) return "u";
  if ("öü".includes(word.lastVowel)) return "ü";
  return "i";
}

function suffix(word: Word, c: Case): string {
  const d = word.hardFinal ? "t" : "d";
  switch (c) {
    case "loc":
      return d + aE(word);
    case "abl":
      return `${d}${aE(word)}n`;
    case "dat":
      return (word.endsInVowel ? "y" : "") + aE(word);
    default: {
      const buffer = { acc: "y", gen: "n", poss: "s" }[c];
      const s = (word.endsInVowel ? buffer : "") + i4(word);
      return c === "gen" ? `${s}n` : s;
    }
  }
}

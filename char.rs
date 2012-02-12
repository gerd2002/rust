#[doc = "Utilities for manipulating the char type"];

/*
    Lu  Uppercase_Letter    an uppercase letter
    Ll  Lowercase_Letter    a lowercase letter
    Lt  Titlecase_Letter    a digraphic character, with first part uppercase
    Lm  Modifier_Letter     a modifier letter
    Lo  Other_Letter    other letters, including syllables and ideographs
    Mn  Nonspacing_Mark     a nonspacing combining mark (zero advance width)
    Mc  Spacing_Mark    a spacing combining mark (positive advance width)
    Me  Enclosing_Mark  an enclosing combining mark
    Nd  Decimal_Number  a decimal digit
    Nl  Letter_Number   a letterlike numeric character
    No  Other_Number    a numeric character of other type
    Pc  Connector_Punctuation   a connecting punctuation mark, like a tie
    Pd  Dash_Punctuation    a dash or hyphen punctuation mark
    Ps  Open_Punctuation    an opening punctuation mark (of a pair)
    Pe  Close_Punctuation   a closing punctuation mark (of a pair)
    Pi  Initial_Punctuation     an initial quotation mark
    Pf  Final_Punctuation   a final quotation mark
    Po  Other_Punctuation   a punctuation mark of other type
    Sm  Math_Symbol     a symbol of primarily mathematical use
    Sc  Currency_Symbol     a currency sign
    Sk  Modifier_Symbol     a non-letterlike modifier symbol
    So  Other_Symbol    a symbol of other type
    Zs  Space_Separator     a space character (of various non-zero widths)
    Zl  Line_Separator  U+2028 LINE SEPARATOR only
    Zp  Paragraph_Separator     U+2029 PARAGRAPH SEPARATOR only
    Cc  Control     a C0 or C1 control code
    Cf  Format  a format control character
    Cs  Surrogate   a surrogate code point
    Co  Private_Use     a private-use character
    Cn  Unassigned  a reserved unassigned code point or a noncharacter
*/

export is_alphabetic,
       is_XID_start, is_XID_continue,
       is_lowercase, is_uppercase,
       is_whitespace, is_alphanumeric,
       is_ascii, is_digit,
       to_digit, to_lower, to_upper, maybe_digit, cmp;

import is_alphabetic = unicode::derived_property::Alphabetic;
import is_XID_start = unicode::derived_property::XID_Start;
import is_XID_continue = unicode::derived_property::XID_Continue;


#[doc(
  brief = "Indicates whether a character is in lower case, defined \
           in terms of the Unicode General Category 'Ll'."
)]
pure fn is_lowercase(c: char) -> bool {
    ret unicode::general_category::Ll(c);
}

#[doc(
  brief = "Indicates whether a character is in upper case, defined \
           in terms of the Unicode General Category 'Lu'."
)]
pure fn is_uppercase(c: char) -> bool {
    ret unicode::general_category::Lu(c);
}

#[doc(
  brief = "Indicates whether a character is whitespace, defined in \
           terms of the Unicode General Categories 'Zs', 'Zl', 'Zp' \
           additional 'Cc'-category control codes in the range [0x09, 0x0d]"
)]
pure fn is_whitespace(c: char) -> bool {
    ret ('\x09' <= c && c <= '\x0d')
        || unicode::general_category::Zs(c)
        || unicode::general_category::Zl(c)
        || unicode::general_category::Zp(c);
}

#[doc(
  brief = "Indicates whether a character is alphanumeric, defined \
            in terms of the Unicode General Categories 'Nd', \
            'Nl', 'No' and the Derived Core Property 'Alphabetic'."
)]
pure fn is_alphanumeric(c: char) -> bool {
    ret unicode::derived_property::Alphabetic(c) ||
        unicode::general_category::Nd(c) ||
        unicode::general_category::Nl(c) ||
        unicode::general_category::No(c);
}

#[doc( brief = "Indicates whether the character is an ASCII character" )]
pure fn is_ascii(c: char) -> bool {
   c - ('\x7F' & c) == '\x00'
}

#[doc( brief = "Indicates whether the character is numeric (Nd, Nl, or No)" )]
pure fn is_digit(c: char) -> bool {
    ret unicode::general_category::Nd(c) ||
        unicode::general_category::Nl(c) ||
        unicode::general_category::No(c);
}

#[doc(
  brief = "Convert a char to the corresponding digit. \
           Safety note: This function fails if `c` is not a valid char",
  return = "If `c` is between '0' and '9', the corresponding value \
            between 0 and 9. If `c` is 'a' or 'A', 10. If `c` is \
            'b' or 'B', 11, etc."
)]
pure fn to_digit(c: char) -> u8 unsafe {
    alt maybe_digit(c) {
      option::some(x) { x }
      option::none { fail; }
    }
}

#[doc(
  brief = "Convert a char to the corresponding digit. Returns none when \
           character is not a valid hexadecimal digit."
)]
pure fn maybe_digit(c: char) -> option<u8> {
    alt c {
      '0' to '9' { option::some(c as u8 - ('0' as u8)) }
      'a' to 'z' { option::some(c as u8 + 10u8 - ('a' as u8)) }
      'A' to 'Z' { option::some(c as u8 + 10u8 - ('A' as u8)) }
      _ { option::none }
    }
}

/*
 FIXME: works only on ASCII
*/
#[doc(
  brief = "Convert a char to the corresponding lower case."
)]
pure fn to_lower(c: char) -> char {
    alt c {
      'A' to 'Z' { ((c as u8) + 32u8) as char }
      _ { c }
    }
}

/*
 FIXME: works only on ASCII
*/
#[doc(
  brief = "Convert a char to the corresponding upper case."
)]
pure fn to_upper(c: char) -> char {
    alt c {
      'a' to 'z' { ((c as u8) - 32u8) as char }
      _ { c }
    }
}

#[doc(
  brief =  "Compare two chars.",
  return = "-1 if a<b, 0 if a==b, +1 if a>b"
)]
pure fn cmp(a: char, b: char) -> int {
    ret  if b > a { -1 }
    else if b < a { 1 }
    else { 0 }
}

#[test]
fn test_is_lowercase() {
    assert is_lowercase('a');
    assert is_lowercase('ö');
    assert is_lowercase('ß');
    assert !is_lowercase('Ü');
    assert !is_lowercase('P');
}

#[test]
fn test_is_uppercase() {
    assert !is_uppercase('h');
    assert !is_uppercase('ä');
    assert !is_uppercase('ß');
    assert is_uppercase('Ö');
    assert is_uppercase('T');
}

#[test]
fn test_is_whitespace() {
    assert is_whitespace(' ');
    assert is_whitespace('\u2007');
    assert is_whitespace('\t');
    assert is_whitespace('\n');

    assert !is_whitespace('a');
    assert !is_whitespace('_');
    assert !is_whitespace('\u0000');
}

#[test]
fn test_to_digit() {
    assert (to_digit('0') == 0u8);
    assert (to_digit('1') == 1u8);
    assert (to_digit('2') == 2u8);
    assert (to_digit('9') == 9u8);
    assert (to_digit('a') == 10u8);
    assert (to_digit('A') == 10u8);
    assert (to_digit('b') == 11u8);
    assert (to_digit('B') == 11u8);
    assert (to_digit('z') == 35u8);
    assert (to_digit('Z') == 35u8);
}

#[test]
#[should_fail]
#[ignore(cfg(target_os = "win32"))]
fn test_to_digit_fail_1() {
    to_digit(' ');
}

#[test]
#[should_fail]
#[ignore(cfg(target_os = "win32"))]
fn test_to_digit_fail_2() {
    to_digit('$');
}

#[test]
fn test_to_lower() {
    assert (to_lower('H') == 'h');
    assert (to_lower('e') == 'e');
    //assert (to_lower('Ö') == 'ö');
    assert (to_lower('ß') == 'ß');
}

#[test]
fn test_to_upper() {
    assert (to_upper('l') == 'L');
    assert (to_upper('Q') == 'Q');
    //assert (to_upper('ü') == 'Ü');
    assert (to_upper('ß') == 'ß');
}

#[test]
fn test_is_ascii() unsafe {
   assert str::all("banana", char::is_ascii);
   assert ! str::all("ประเทศไทย中华Việt Nam", char::is_ascii);
}

#[test]
fn test_is_digit() {
   assert is_digit('2');
   assert is_digit('7');
   assert ! is_digit('c');
   assert ! is_digit('i');
   assert ! is_digit('z');
   assert ! is_digit('Q');
}


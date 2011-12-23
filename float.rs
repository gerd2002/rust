/*
Module: float
*/

// PORT This must match in width according to architecture

import m_float = f64;
import m_float::*;

type t = float;

/**
 * Section: String Conversions
 */

/*
Function: to_str_common

Converts a float to a string

Parameters:

num - The float value
digits - The number of significant digits
exact - Whether to enforce the exact number of significant digits
*/
fn to_str_common(num: float, digits: uint, exact: bool) -> str {
    let (num, accum) = num < 0.0 ? (-num, "-") : (num, "");
    let trunc = num as uint;
    let frac = num - (trunc as float);
    accum += uint::str(trunc);
    if frac == 0.0 || digits == 0u { ret accum; }
    accum += ".";
    let i = digits;
    let epsilon = 1. / pow_uint_to_uint_as_float(10u, i);
    while i > 0u && (frac >= epsilon || exact) {
        frac *= 10.0;
        epsilon *= 10.0;
        let digit = frac as uint;
        accum += uint::str(digit);
        frac -= digit as float;
        i -= 1u;
    }
    ret accum;

}

/*
Function: to_str

Converts a float to a string with exactly the number of provided significant
digits

Parameters:

num - The float value
digits - The number of significant digits
*/
fn to_str_exact(num: float, digits: uint) -> str {
    to_str_common(num, digits, true)
}

/*
Function: to_str

Converts a float to a string with a maximum number of significant digits

Parameters:

num - The float value
digits - The number of significant digits
*/
fn to_str(num: float, digits: uint) -> str {
    to_str_common(num, digits, false)
}

/*
Function: from_str

Convert a string to a float

This function accepts strings such as
* "3.14"
* "+3.14", equivalent to "3.14"
* "-3.14"
* "2.5E10", or equivalently, "2.5e10"
* "2.5E-10"
* "", or, equivalently, "." (understood as 0)
* "5."
* ".5", or, equivalently,  "0.5"

Leading and trailing whitespace are ignored.

Parameters:

num - A string, possibly empty.

Returns:

<NaN> If the string did not represent a valid number.
Otherwise, the floating-point number represented [num].
*/
fn from_str(num: str) -> float {
   let num = str::trim(num);

   let pos = 0u;                  //Current byte position in the string.
                                  //Used to walk the string in O(n).
   let len = str::byte_len(num);  //Length of the string, in bytes.

   if len == 0u { ret 0.; }
   let total = 0f;                //Accumulated result
   let c     = 'z';               //Latest char.

   //The string must start with one of the following characters.
   alt str::char_at(num, 0u) {
      '-' | '+' | '0' to '9' | '.' {}
      _ { ret NaN; }
   }

   //Determine if first char is '-'/'+'. Set [pos] and [neg] accordingly.
   let neg = false;               //Sign of the result
   alt str::char_at(num, 0u) {
      '-' {
          neg = true;
          pos = 1u;
      }
      '+' {
          pos = 1u;
      }
      _ {}
   }

   //Examine the following chars until '.', 'e', 'E'
   while(pos < len) {
       let char_range = str::char_range_at(num, pos);
       c   = char_range.ch;
       pos = char_range.next;
       alt c {
         '0' to '9' {
           total = total * 10f;
           total += ((c as int) - ('0' as int)) as float;
         }
         '.' | 'e' | 'E' {
           break;
         }
         _ {
           ret NaN;
         }
       }
   }

   if c == '.' {//Examine decimal part
      let decimal = 1f;
      while(pos < len) {
         let char_range = str::char_range_at(num, pos);
         c = char_range.ch;
         pos = char_range.next;
         alt c {
            '0' | '1' | '2' | '3' | '4' | '5' | '6'| '7' | '8' | '9'  {
                 decimal /= 10f;
                 total += (((c as int) - ('0' as int)) as float)*decimal;
             }
             'e' | 'E' {
                 break;
             }
             _ {
                 ret NaN;
             }
         }
      }
   }

   if (c == 'e') | (c == 'E') {//Examine exponent
      let exponent = 0u;
      let neg_exponent = false;
      if(pos < len) {
          let char_range = str::char_range_at(num, pos);
          c   = char_range.ch;
          alt c  {
             '+' {
                pos = char_range.next;
             }
             '-' {
                pos = char_range.next;
                neg_exponent = true;
             }
             _ {}
          }
          while(pos < len) {
             let char_range = str::char_range_at(num, pos);
             c = char_range.ch;
             alt c {
                 '0' | '1' | '2' | '3' | '4' | '5' | '6'| '7' | '8' | '9' {
                     exponent *= 10u;
                     exponent += ((c as uint) - ('0' as uint));
                 }
                 _ {
                     break;
                 }
             }
             pos = char_range.next;
          }
          let multiplier = pow_uint_to_uint_as_float(10u, exponent);
              //Note: not [int::pow], otherwise, we'll quickly
              //end up with a nice overflow
          if neg_exponent {
             total = total / multiplier;
          } else {
             total = total * multiplier;
          }
      } else {
         ret NaN;
      }
   }

   if(pos < len) {
     ret NaN;
   } else {
     if(neg) {
        total *= -1f;
     }
     ret total;
   }
}

/**
 * Section: Arithmetics
 */

/*
Function: pow_uint_to_uint_as_float

Compute the exponentiation of an integer by another integer as a float.

Parameters:
x - The base.
pow - The exponent.

Returns:
<NaN> of both `x` and `pow` are `0u`, otherwise `x^pow`.
*/
fn pow_uint_to_uint_as_float(x: uint, pow: uint) -> float {
   if x == 0u {
      if pow == 0u {
        ret NaN;
      }
       ret 0.;
   }
   let my_pow     = pow;
   let total      = 1f;
   let multiplier = x as float;
   while (my_pow > 0u) {
     if my_pow % 2u == 1u {
       total = total * multiplier;
     }
     my_pow     /= 2u;
     multiplier *= multiplier;
   }
   ret total;
}


/*
Function: min

Returns the minimum of two values
*/
pure fn min<copy T>(x: T, y: T) -> T { x < y ? x : y }

/*
Function: max

Returns the maximum of two values
*/
pure fn max<copy T>(x: T, y: T) -> T { x < y ? y : x }

//
// Local Variables:
// mode: rust
// fill-column: 78;
// indent-tabs-mode: nil
// c-basic-offset: 4
// buffer-file-coding-system: utf-8-unix
// End:
//






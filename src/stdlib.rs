//! The Rant standard library.

#![allow(unused_variables)]

use std::rc::Rc;
use crate::*;
use crate::runtime::*;
use crate::convert::*;
use crate::convert::ToRant;

mod assert;
mod block;
mod boolean;
mod collections;
mod compare;
mod control;
mod convert;
mod format;
mod general;
mod generate;
mod math;
mod proto;
mod strings;
mod verify;

use self::{
  assert::*, block::*, boolean::*, collections::*, 
  compare::*, control::*, convert::*, format::*, 
  general::*, generate::*, math::*, proto::*, 
  strings::*, verify::*
};

pub(crate) type RantStdResult = Result<(), RuntimeError>;

#[macro_export]
macro_rules! runtime_error {
  ($err_type:expr, $msg:literal) => {
    return Err(RuntimeError {
      error_type: $err_type,
      description: $msg.to_owned(),
      stack_trace: None,
    })
  };
  ($err_type:expr, $msg_fmt:literal, $($msg_fmt_args:expr),+) => {
    return Err(RuntimeError {
      error_type: $err_type,
      description: format!($msg_fmt, $($msg_fmt_args),+),
      stack_trace: None,
    })
  };
}

pub(crate) fn load_stdlib(context: &mut Rant)
{
  macro_rules! load_func {
    ($fname:ident) => {{
      let func = $fname.as_rant_func();
      context.set_global(stringify!($fname), RantValue::Function(Rc::new(func)));
    }};
    ($fname:ident, $id:literal) => {{
      let func = $fname.as_rant_func();
      context.set_global($id, RantValue::Function(Rc::new(func)));
    }};
  }

  macro_rules! load_funcs {
    ($($fname:ident $(as $id:expr)?),+) => {
      $(load_func!($fname$(, $id)?);)+
    };
  }

  load_funcs!(
    // General functions
    alt, call, either, len, get_type as "type", seed, nop, resolve, fork, unfork,

    // Assertion functions
    _assert as "assert", _assert_eq as "assert-eq", _assert_neq as "assert-neq",

    // Formatting functions
    whitespace_fmt as "whitespace-fmt",

    // Block attribute / control flow functions
    break_ as "break", continue_ as "continue", if_ as "if", else_if as "else-if", else_ as "else", 
    mksel, rep, return_ as "return", sel, sep,

    // Attribute frame stack functions
    push_attrs as "push-attrs", pop_attrs as "pop-attrs", count_attrs as "count-attrs", reset_attrs as "reset-attrs",

    // Block state functions
    step, step_index as "step-index", step_count as "step-count",

    // Boolean functions
    and, not, or, xor,

    // Comparison functions
    eq, neq, gt, lt, ge, le,

    // Verification functions
    is_string as "is-string", is_integer as "is-integer", is_float as "is-float", 
    is_number as "is-number", is_bool as "is-bool", is_empty as "is-empty", is_nan as "is-nan",
    is_between as "is-between", is_any as "is-any", is,

    // Math functions
    add, sub, mul, div, mul_add as "mul-add", mod_ as "mod", neg, recip, is_odd as "is-odd", is_even as "is-even", is_factor as "is-factor",
    clamp,

    // Conversion functions
    to_int as "int", to_float as "float", to_string as "string",

    // Generator functions
    alpha, dig, digh, dignz, maybe, rand, randf, rand_list as "rand-list", randf_list as "randf-list", shred,

    // Prototype functions
    proto, set_proto as "set-proto",

    // Collection functions
    assoc, clear, has, keys, index_of as "index-of", insert, last_index_of as "last-index-of", remove, sift, sifted, squish, squished, take, translate,

    // List functions
    pick, filter, join, map, sort, sorted, shuffle, shuffled, sum, min, max,
    list_push as "push", list_pop as "pop", oxford_join as "oxford-join", zip,

    // String functions
    lower, upper, seg, split, lines, indent,

    // Error functions
    error
  );

  // Load [require] function if requested
  if context.options.enable_require {
    load_func!(require);
  }

  // Miscellaneous
  context.set_global("RANT_VERSION", RantValue::String(RANT_VERSION.to_owned()));
}
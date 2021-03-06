use super::*;

/// `[$and: lhs (bool); rhs (bool); extra* (bool)]`
///
/// Returns the logical AND of the operands.
pub(crate) fn and(vm: &mut VM, (lhs, rhs, extra): (bool, bool, VarArgs<bool>)) -> RantStdResult {
  let result = (lhs && rhs) && extra.iter().all(|b| *b);
  vm.cur_frame_mut().write_value(RantValue::Boolean(result));
  Ok(())
}

/// `[$or: lhs (bool); rhs (bool); extra* (bool)]`
///
/// Returns the logical OR of the operands.
pub(crate) fn or(vm: &mut VM, (lhs, rhs, extra): (bool, bool, VarArgs<bool>)) -> RantStdResult {
  let result = (lhs || rhs) || extra.iter().any(|b| *b);
  vm.cur_frame_mut().write_value(RantValue::Boolean(result));
  Ok(())
}

/// `[$not: val (bool)]`
///
/// Gets the inverse of the operand.
pub(crate) fn not(vm: &mut VM, val: bool) -> RantStdResult {
  vm.cur_frame_mut().write_value(RantValue::Boolean(!val));
  Ok(())
}

/// `$xor: lhs (bool); rhs (bool)]`
///
/// Retirms the logical XOR of the operands.
pub(crate) fn xor(vm: &mut VM, (lhs, rhs): (bool, bool)) -> RantStdResult {
  vm.cur_frame_mut().write_value(RantValue::Boolean(lhs ^ rhs));
  Ok(())
}
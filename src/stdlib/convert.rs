use super::*;

pub(crate) fn to_int(vm: &mut VM, value: RantValue) -> RantStdResult {
  vm.cur_frame_mut().write_value(value.into_rant_int());
  Ok(())
}

pub(crate) fn to_float(vm: &mut VM, value: RantValue) -> RantStdResult {
  vm.cur_frame_mut().write_value(value.into_rant_float());
  Ok(())
}

pub(crate) fn to_string(vm: &mut VM, value: RantValue) -> RantStdResult {
  vm.cur_frame_mut().write_value(value.into_rant_string());
  Ok(())
}
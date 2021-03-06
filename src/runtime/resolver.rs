use std::{cell::RefCell, rc::Rc, mem, error::Error, fmt::Display};
use crate::{random::RantRng, RantValue, lang::{Sequence, Block, PrintFlag}, FromRant, ValueError};
use smallvec::SmallVec;
use super::{IntoRuntimeResult, RuntimeError};

pub type SelectorRef = Rc<RefCell<Selector>>;

/// The number of attribute frames you can put on the stack before the runtime goes up in smoke.
const DEFAULT_MAX_ATTR_FRAMES: usize = 127;
const BLOCK_STACK_INLINE_COUNT: usize = 4;

/// Manages block execution behavior ("resolution").
pub struct Resolver {
  rng: Rc<RantRng>,
  base_attrs: AttributeFrame,
  attr_override_stack: Vec<AttributeFrame>,
  block_stack: SmallVec<[BlockState; BLOCK_STACK_INLINE_COUNT]>,
}

/// Stores state information for a block that is currently being resolved.
#[derive(Debug)]
pub struct BlockState {
  elements: Rc<Vec<Rc<Sequence>>>,
  force_stop: bool,
  flag: PrintFlag,
  attrs: AttributeFrame,
  cur_steps: usize,
  total_steps: usize,
  prev_step_separated: bool,
}

impl BlockState {
  #[inline]
  pub fn next_element(&mut self, rng: &RantRng) -> Result<Option<BlockAction>, SelectorError> {
    if !self.is_done() {
      if self.cur_steps == 0 || self.prev_step_separated {
        self.prev_step_separated = false;
        self.cur_steps += 1;
        let next_index = self.attrs.selector.as_ref().map_or_else(
          // Default block selection behavior
          || Ok(rng.next_usize(self.elements.len())), 
          // Selector behavior
          |sel| sel.borrow_mut().select(self.elements.len(), rng)
        )?;
        Ok(Some(BlockAction::Element(Rc::clone(&self.elements[next_index]))))
      } else {
        self.prev_step_separated = true;
        Ok(Some(BlockAction::Separator(self.attrs.separator.clone())))
      }
    } else {
      Ok(None)
    }
  }

  #[inline(always)]
  pub fn force_stop(&mut self) {
    self.force_stop = true;
  }

  #[inline]
  pub fn step_index(&self) -> usize {
    self.cur_steps - 1
  }

  #[inline]
  pub fn step(&self) -> usize {
    self.cur_steps
  }

  #[inline]
  pub fn step_count(&self) -> usize {
    self.total_steps
  }

  #[inline]
  pub fn is_repeater(&self) -> bool {
    matches!(self.attrs.reps, Reps::Repeat(_) | Reps::RepeatForever | Reps::All)
  }

  /// Indicates whether the block has finished and should return.
  #[inline(always)]
  pub(crate) fn is_done(&self) -> bool {
    // Force-stop from break
    self.force_stop
    // Conditional value has evaluated to false
    || !self.attrs.condval.unwrap_or(true) 
    // Finite repetitions are exhausted
    || (!self.attrs.reps.is_infinite() && self.cur_steps >= self.total_steps)
  }

  #[inline(always)]
  pub fn flag(&self) -> PrintFlag {
    self.flag
  }
}

pub enum BlockAction {
  Element(Rc<Sequence>),
  Separator(RantValue),
}

#[derive(Debug, Copy, Clone)]
pub enum Reps {
  /// Repeat forever.
  RepeatForever,
  /// Iterate as many times as there are elements in the block.
  All,
  /// Iterate a specific number of times.
  Repeat(usize),
  // Resolve once.
  Once,
}

impl Reps {
  #[inline(always)]
  pub fn is_infinite(&self) -> bool {
    matches!(self, Reps::RepeatForever)
  }

  #[inline(always)]
  pub fn is_all(&self) -> bool {
    matches!(self, Reps::All)
  }

  #[inline]
  pub fn get_rep_count_for(&self, block: &Block) -> usize {
    match self {
      Reps::RepeatForever => 0,
      Reps::Once => 1,
      Reps::All => block.len(),
      Reps::Repeat(n) => *n,
    }
  }
}

impl Resolver {
  pub fn new(rng: &Rc<RantRng>) -> Self {
    Self {
      rng: rng.clone(),
      base_attrs: Default::default(),
      attr_override_stack: vec![Default::default()],
      block_stack: Default::default(),
    }
  }
}

impl Resolver {
  /// Adds a new block state to the block stack.
  #[inline]
  pub fn push_block(&mut self, block: &Block, flag: PrintFlag) {
    let attrs = self.take_attrs();
    let state = BlockState {
      elements: Rc::clone(&block.elements),
      flag: PrintFlag::prioritize(block.flag, flag),
      cur_steps: 0,
      total_steps: attrs.reps.get_rep_count_for(block),
      attrs,
      prev_step_separated: false,
      force_stop: false,
    };
    // Since blocks are associated with call stack frames, there is no need to check the stack size here
    self.block_stack.push(state);
  }

  /// Removes the active block state from the block stack.
  #[inline]
  pub fn pop_block(&mut self) -> Option<BlockState> {
    self.block_stack.pop()
  }

  /// Gets a reference to the active block state.
  #[inline]
  pub fn active_block(&self) -> Option<&BlockState> {
    self.block_stack.last()
  }

  /// Gets a mutable reference to the active block state.
  #[inline]
  pub fn active_block_mut(&mut self) -> Option<&mut BlockState> {
    self.block_stack.last_mut()
  }

  /// Gets a mutable reference to the active repeater state.
  #[inline]
  pub fn active_repeater_mut(&mut self) -> Option<&mut BlockState> {
    self.block_stack
      .iter_mut()
      .rev()
      .find(|b| b.is_repeater())
  }

  /// Gets a reference to the active repeater state.
  #[inline]
  pub fn active_repeater(&self) -> Option<&BlockState> {
    self.block_stack
      .iter()
      .rev()
      .find(|b| b.is_repeater())
  }

  /// Takes the topmost attribute frame for use elsewhere and replaces it with a default one.
  #[inline]
  pub fn take_attrs(&mut self) -> AttributeFrame {
    if self.attr_override_stack.is_empty() {
      let next_attr = AttributeFrame::propagate(&self.base_attrs);
      mem::replace(&mut self.base_attrs, next_attr)
    } else {
      let last_attr = self.attr_override_stack.last_mut().unwrap();
      let next_attr = AttributeFrame::propagate(last_attr);
      mem::replace(last_attr, next_attr)
    }
  }

  #[inline]
  pub fn reset_attrs(&mut self) {
    if self.attr_override_stack.is_empty() {
      mem::take(&mut self.base_attrs);
    } else {
      mem::take(self.attr_override_stack.last_mut().unwrap());
    }
  }

  pub fn push_attrs(&mut self) {
    // TODO: Limit attr frame stack size
    self.attr_override_stack.push(Default::default())
  }

  pub fn pop_attrs(&mut self) -> Option<AttributeFrame> {
    self.attr_override_stack.pop()
  }

  pub fn count_attrs(&self) -> usize {
    self.attr_override_stack.len() + 1
  }

  #[inline]
  pub fn attrs(&self) -> &AttributeFrame {
    if self.attr_override_stack.is_empty() {
      &self.base_attrs
    } else {
      self.attr_override_stack.last().unwrap()
    }
  }

  #[inline]
  pub fn attrs_mut(&mut self) -> &mut AttributeFrame {
    if self.attr_override_stack.is_empty() {
      &mut self.base_attrs
    } else {
      self.attr_override_stack.last_mut().unwrap()
    }
  }
}

/// A full set of block attributes.
#[derive(Debug)]
pub struct AttributeFrame {
  /// Conditional value returned from last [if]-like call
  pub condval: Option<bool>,
  /// Conditional value used by previous block
  pub prev_condval: Option<bool>,
  /// Indicates if the next attribute frame should receive the current condval
  pub no_propagate_condval: bool,
  /// Repetition value
  pub reps: Reps,
  /// Separator value
  pub separator: RantValue,
  /// Active selector
  pub selector: Option<SelectorRef>,
}

impl AttributeFrame {
  /// Creates a new frame, propagating the condval of the specified frame if able.
  pub fn propagate(frame: &AttributeFrame) -> Self {
    Self {
      prev_condval: if frame.no_propagate_condval { None } else { frame.condval },
      .. Default::default()
    }
  }

  #[inline]
  pub fn make_if(&mut self, cond_val: bool) {
    self.condval = Some(cond_val);
    // Propagate if false
    self.no_propagate_condval = cond_val;
  }

  #[inline]
  pub fn make_else(&mut self) {
    self.condval = Some(self.prev_condval.map(|b| !b).unwrap_or(false));
    // Do not propagate condvals for else-clauses
    self.no_propagate_condval = true;
  }

  #[inline]
  pub fn make_else_if(&mut self, cond_val: bool) {
    // Check if there's a condval propagated from a previous block
    let has_propagated_condval = self.prev_condval.is_none();
    // Previous condval must be false and current condval true for clause to run
    // If there is no previous condval, add false non-propagating condval
    self.condval = Some(self.prev_condval.map(|b| !b && cond_val).unwrap_or(false));
    // Only propagate condval if it is false
    self.no_propagate_condval = cond_val || has_propagated_condval;
  }
}

impl Default for AttributeFrame {
  fn default() -> Self {
    Self {
      condval: None,
      prev_condval: None,
      no_propagate_condval: false,
      reps: Reps::Once,
      separator: RantValue::Empty,
      selector: None,
    }
  }
}

#[derive(Debug)]
pub struct Selector {
  /// Mode of the selector
  mode: SelectorMode,
  /// Current iteration of the selector
  index: usize,
  /// Element count of the selector
  count: usize,
  /// True if the pass is odd (used by ping/pong)
  parity: bool,
  /// Jump table used by some selector modes (won't allocate if unused)
  jump_table: Vec<usize>,
}

impl Selector {
  #[inline]
  pub fn new(mode: SelectorMode) -> Self {
    Self {
      mode,
      index: 0,
      count: 0,
      parity: false,
      jump_table: Default::default(),
    }
  }

  #[inline]
  pub fn is_initialized(&self) -> bool {
    self.count > 0
  }

  #[inline]
  pub fn init(&mut self, rng: &RantRng, elem_count: usize) -> Result<(), SelectorError> {
    if elem_count == 0 {
      return Err(SelectorError::InvalidElementCount(0))
    }

    self.count = elem_count;
    
    match self.mode {
      SelectorMode::Random | SelectorMode::One => {
        self.index = rng.next_usize(elem_count);
      },
      SelectorMode::Forward => {},
      SelectorMode::ForwardClamp => {},
      SelectorMode::Reverse | SelectorMode::ReverseClamp => {
        self.index = elem_count - 1;
      },
      SelectorMode::Deck | SelectorMode::DeckLoop | SelectorMode::DeckClamp => {
        self.shuffle(rng);
      },
      SelectorMode::Ping => {},
      SelectorMode::Pong => {
        self.index = elem_count - 1;
      },
      SelectorMode::NoDouble => {
        self.index = rng.next_usize(elem_count);
      },
    }

    Ok(())
  }

  #[inline]
  fn shuffle(&mut self, rng: &RantRng) {
    let jump_table = &mut self.jump_table;
    let n = self.count;

    // Populate the jump table if it isn't already
    if jump_table.is_empty() {
      jump_table.reserve(n);
      jump_table.extend(0..n);
    }

    // Perform a Fisher-Yates shuffle
    for i in 0..n {
      jump_table.swap(i, rng.next_usize(n));
    }
  }

  pub fn select(&mut self, elem_count: usize, rng: &RantRng) -> Result<usize, SelectorError> {
    // Initialize and sanity check
    if !self.is_initialized() {
      self.init(rng, elem_count)?;
    } else if elem_count != self.count {
      return Err(SelectorError::ElementCountMismatch { 
        expected: self.count,
        found: elem_count,
      })
    }

    let cur_index = self.index;

    // Iterate the selector
    match self.mode {
      SelectorMode::Random => {
        self.index = rng.next_usize(elem_count);
      },
      SelectorMode::One => {},
      SelectorMode::Forward => {
        self.index = (cur_index + 1) % elem_count;
      },
      SelectorMode::ForwardClamp => {
        self.index = (cur_index + 1).min(elem_count - 1);
      },
      SelectorMode::Reverse => {
        self.index = if cur_index == 0 {
          elem_count
        } else {
          cur_index
        } - 1;
      },
      SelectorMode::ReverseClamp => {
        self.index = cur_index.saturating_sub(1);
      },
      SelectorMode::Deck => {
        // Store the return value before reshuffling to avoid accidental early duplicates
        let jump_index = self.jump_table[cur_index];

        if cur_index >= elem_count - 1 {
          self.shuffle(rng);
          self.index = 0;
        } else {
          self.index = cur_index + 1;
        }

        return Ok(jump_index)
      },
      SelectorMode::DeckLoop => {
        self.index = (cur_index + 1) % elem_count;
        return Ok(self.jump_table[cur_index])
      },
      SelectorMode::DeckClamp => {
        self.index = (cur_index + 1).min(elem_count - 1);
        return Ok(self.jump_table[cur_index])
      },
      SelectorMode::Ping => {
        let prev_parity = self.parity;
        if (prev_parity && cur_index == 0) || (!prev_parity && cur_index == elem_count - 1) {
          self.parity = !prev_parity;
        }

        if self.parity {
          self.index = cur_index.saturating_sub(1);
        } else {
          self.index = (cur_index + 1) % elem_count;
        }
      },
      SelectorMode::Pong => {
        let prev_parity = self.parity;
        if (!prev_parity && cur_index == 0) || (prev_parity && cur_index == elem_count - 1) {
          self.parity = !prev_parity;
        }

        if self.parity {
          self.index = (cur_index + 1) % elem_count;
        } else {
          self.index = cur_index.saturating_sub(1);
        }
      },
      SelectorMode::NoDouble => {
        self.index = if elem_count > 1 {
          (cur_index + 1 + rng.next_usize(elem_count - 1)) % elem_count
        } else {
          0
        };
      },
    }

    Ok(cur_index)
  }
}

#[derive(Debug)]
pub enum SelectorError {
  ElementCountMismatch { expected: usize, found: usize },
  InvalidElementCount(usize),
}

impl Error for SelectorError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    None
  }

  fn cause(&self) -> Option<&dyn Error> {
    self.source()
  }
}

impl Display for SelectorError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SelectorError::ElementCountMismatch { expected, found } => write!(f, "selector expected {} elements, but found {}", expected, found),
      SelectorError::InvalidElementCount(n) => write!(f, "selector does not support blocks of size {}", n),
    }
  }
}

impl<T> IntoRuntimeResult<T> for Result<T, SelectorError> {
  fn into_runtime_result(self) -> super::RuntimeResult<T> {
    self.map_err(|err| RuntimeError {
      description: err.to_string(),
      error_type: super::RuntimeErrorType::SelectorError(err),
      stack_trace: None,
    })
  }
}

#[derive(Debug)]
#[repr(u8)]
pub enum SelectorMode {
  /// Selects a random element each time.
  Random,
  /// Selects the same, random element each time.
  One,
  /// Selects each element in a wrapping sequence from left to right.
  Forward,
  /// Selects each element from left to right, then repeats the right-most element.
  ForwardClamp,
  /// Selects each element in a wrapping reverse sequence from right to left.
  Reverse,
  /// Selects each element from right to left, then repeats the left-most element.
  ReverseClamp,
  /// Selects each element once in a random sequence, then reshuffles.
  Deck,
  /// Selects each element once in a wrapping random sequence, without reshuffling.
  DeckLoop,
  /// Selects each element once in a random sequence, repeating the final element.
  DeckClamp,
  /// Selects each element from left to right, switching directions each time an edge element is reached.
  Ping,
  /// Selects each element from right to left, switching directions each time an edge element is reached.
  Pong,
  /// Ensures that no one element index is selected twice in a row.
  NoDouble,
}

impl FromRant for SelectorMode {
  fn from_rant(val: RantValue) -> Result<Self, ValueError> {
    match &val {
      RantValue::String(s) => {
        Ok(match s.as_str() {
          "random" =>         SelectorMode::Random,
          "one" =>            SelectorMode::One,
          "forward" =>        SelectorMode::Forward,
          "forward-clamp" =>  SelectorMode::ForwardClamp,
          "reverse" =>        SelectorMode::Reverse,
          "reverse-clamp" =>  SelectorMode::ReverseClamp,
          "deck" =>           SelectorMode::Deck,
          "deck-loop" =>      SelectorMode::DeckLoop,
          "deck-clamp" =>     SelectorMode::DeckClamp,
          "ping" =>           SelectorMode::Ping,
          "pong" =>           SelectorMode::Pong,
          "no-double" =>      SelectorMode::NoDouble,
          _ => return Err(ValueError::InvalidConversion {
            from: val.type_name(),
            to: "selector mode",
            message: Some(format!("invalid selector mode: '{}'", s))
          })
        })
      },
      _ => Err(ValueError::InvalidConversion {
        from: val.type_name(),
        to: "selector mode",
        message: None,
      })
    }
  }

  fn is_rant_optional() -> bool {
    false
  }
}
use std::{rc::Rc, ops::{DerefMut, Deref}, iter::FromIterator, cell::RefCell};
use crate::{RantString, RantValue};
use fnv::FnvHashMap;

const DEFAULT_MAP_CAPACITY: usize = 16;
const DEFAULT_LIST_CAPACITY: usize = 16;

pub type RantMapRef = Rc<RefCell<RantMap>>;
pub type RantListRef = Rc<RefCell<RantList>>;

/// Represents Rant's `list` type, which stores an ordered collection of values.
#[derive(Debug, Clone)]
pub struct RantList(Vec<RantValue>);

impl RantList {
  /// Creates an empty RantList.
  pub fn new() -> Self {
    Self(Vec::with_capacity(DEFAULT_LIST_CAPACITY))
  }

  /// Creates an empty RantList with the specified initial capacity.
  pub fn with_capacity(capacity: usize) -> Self {
    Self(Vec::with_capacity(capacity))
  }
}

impl From<Vec<RantValue>> for RantList {
  fn from(list: Vec<RantValue>) -> Self {
    Self(list)
  }
}

impl Default for RantList {
  fn default() -> Self {
    Self::new()
  }
}

impl Deref for RantList {
  type Target = Vec<RantValue>;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl DerefMut for RantList {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl FromIterator<RantValue> for RantList {
  fn from_iter<T: IntoIterator<Item = RantValue>>(iter: T) -> Self {
    let mut list = Self::new();
    for item in iter {
      list.push(item);
    }
    list
  }
}

impl IntoIterator for RantList {
  type Item = RantValue;
  type IntoIter = std::vec::IntoIter<Self::Item>;

  fn into_iter(self) -> Self::IntoIter {
    self.0.into_iter()
  }
}

/// Represents Rant's `map` type, which stores a collection of key-value pairs.
/// Map keys are always strings.
#[derive(Debug, Clone)]
pub struct RantMap {
  /// The physical contents of the map
  map: FnvHashMap<RantString, RantValue>,
  /// The prototype of the map
  proto: Option<RantMapRef>
}

impl RantMap {
  pub fn new() -> Self {
    Self {
      map: FnvHashMap::with_capacity_and_hasher(DEFAULT_MAP_CAPACITY, Default::default()),
      proto: None
    }
  }

  #[inline]
  pub fn clear(&mut self) {
    self.map.clear();
  }

  #[inline]
  pub fn raw_len(&self) -> usize {
    self.map.len()
  }
  
  #[inline]
  pub fn is_empty(&self) -> bool {
    self.map.is_empty()
  }

  #[inline]
  pub fn proto(&self) -> Option<RantMapRef> {
    self.proto.clone()
  }

  #[inline]
  pub fn set_proto(&mut self, proto: Option<RantMapRef>) {
    self.proto = proto;
  }

  #[inline]
  pub fn raw_set(&mut self, key: &str, val: RantValue) {
    self.map.insert(RantString::from(key), val);
  }

  #[inline]
  pub fn raw_remove(&mut self, key: &str) {
    self.map.remove(key);
  }

  #[inline]
  pub fn raw_take(&mut self, key: &str) -> Option<RantValue> {
    self.map.remove(key)
  }

  #[inline]
  pub fn raw_get<'a>(&'a self, key: &str) -> Option<&'a RantValue> {
    self.map.get(key)
  }

  #[inline]
  pub fn raw_has_key(&self, key: &str) -> bool {
    self.map.contains_key(key)
  }

  #[inline]
  pub fn raw_keys(&self) -> RantList {
    RantList::from_iter(self.map.keys().map(|k| RantValue::String(k.to_string())))
  }
}

impl Default for RantMap {
  fn default() -> Self {
    RantMap::new()
  }
}
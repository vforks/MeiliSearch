use crate::{FieldsMap, FieldId, SResult, Error, IndexedPos};
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::borrow::Cow;

#[derive(Clone, Debug, Serialize, Deserialize)]
enum OptionAll<T> {
    All,
    Some(T),
    None,
}

impl<T> OptionAll<T> {
    // replace the value with None and return the previous value
    fn take(&mut self) -> OptionAll<T> {
        std::mem::replace(self, OptionAll::None)
    }

    pub fn is_all(&self) -> bool {
        matches!(self, OptionAll::All)
    }
}

impl<T> Default for OptionAll<T> {
    fn default() -> OptionAll<T> {
        OptionAll::All
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Schema {
    fields_map: FieldsMap,

    primary_key: Option<FieldId>,
    ranked: HashSet<FieldId>,
    displayed: OptionAll<HashSet<FieldId>>,

    indexed: OptionAll<HashSet<FieldId>>,
    fields_position: HashMap<FieldId, IndexedPos>,
}

impl Schema {
    pub fn new() -> Schema {
        Schema::default()
    }

    pub fn with_primary_key(name: &str) -> Schema {
        let mut fields_map = FieldsMap::default();
        let field_id = fields_map.insert(name).unwrap();

        let mut displayed = HashSet::new();
        let mut indexed_map = HashMap::new();

        displayed.insert(field_id);
        indexed_map.insert(field_id, 0.into());

        Schema {
            fields_map,
            primary_key: Some(field_id),
            ranked: HashSet::new(),
            displayed: OptionAll::All,
            indexed: OptionAll::All,
            fields_position: indexed_map,
        }
    }

    pub fn primary_key(&self) -> Option<&str> {
        self.primary_key.map(|id| self.fields_map.name(id).unwrap())
    }

    pub fn set_primary_key(&mut self, name: &str) -> SResult<FieldId> {
        if self.primary_key.is_some() {
            return Err(Error::PrimaryKeyAlreadyPresent)
        }

        let id = self.insert(name)?;
        self.primary_key = Some(id);
        self.set_indexed(name)?;
        self.set_displayed(name)?;

        Ok(id)
    }

    pub fn id(&self, name: &str) -> Option<FieldId> {
        self.fields_map.id(name)
    }

    pub fn name<I: Into<FieldId>>(&self, id: I) -> Option<&str> {
        self.fields_map.name(id)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.fields_map.iter().map(|(k, _)| k.as_ref())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.fields_map.id(name).is_some()
    }

    pub fn insert(&mut self, name: &str) -> SResult<FieldId> {
        self.fields_map.insert(name)
    }

    pub fn register_field(&mut self, name: &str) -> SResult<FieldId> {
        match self.fields_map.id(name) {
            Some(id) => {
                Ok(id)
            }
            None => {
                let id = self.fields_map.insert(name)?;
                let pos = self.fields_position.len() as u16;
                self.fields_position.insert(id, pos.into());
                Ok(id)
            }
        }
    }

    pub fn ranked(&self) -> &HashSet<FieldId> {
        &self.ranked
    }

    pub fn ranked_name(&self) -> HashSet<&str> {
        self.ranked.iter().filter_map(|a| self.name(*a)).collect()
    }

    pub fn displayed(&self) -> Cow<HashSet<FieldId>> {
        match self.displayed {
            OptionAll::Some(ref v) => Cow::Borrowed(v),
            OptionAll::All => {
                let fields = self
                    .fields_map
                    .iter()
                    .map(|(_, &v)| v)
                    .collect::<HashSet<_>>();
                Cow::Owned(fields)
            }
            OptionAll::None => Cow::Owned(HashSet::new())
        }
    }

    pub fn is_displayed_all(&self) -> bool {
        self.displayed.is_all()
    }

    pub fn displayed_name(&self) -> HashSet<&str> {
        match self.displayed {
            OptionAll::All => self.fields_map.iter().filter_map(|(_, &v)| self.name(v)).collect(),
            OptionAll::Some(ref v) => v.iter().filter_map(|a| self.name(*a)).collect(),
            OptionAll::None => HashSet::new(),
        }
    }

    pub fn indexed(&self) -> Vec<FieldId> {
        match self.indexed {
            OptionAll::Some(ref v) => v.iter().cloned().collect(),
            OptionAll::All => {
                let fields = self
                    .fields_position
                    .keys()
                    .cloned()
                    .collect();
                fields
            },
            OptionAll::None => Vec::new()
        }
    }

    pub fn indexed_name(&self) -> Vec<&str> {
        self.indexed().iter().filter_map(|a| self.name(*a)).collect()
    }

    pub fn set_ranked(&mut self, name: &str) -> SResult<FieldId> {
        let id = self.fields_map.insert(name)?;
        self.ranked.insert(id);
        Ok(id)
    }

    pub fn set_displayed(&mut self, name: &str) -> SResult<FieldId> {
        let id = self.fields_map.insert(name)?;
        self.displayed = match self.displayed.take() {
            OptionAll::All => OptionAll::All,
            OptionAll::None => {
                let mut displayed = HashSet::new();
                displayed.insert(id);
                OptionAll::Some(displayed)
            },
            OptionAll::Some(mut v) => {
                v.insert(id);
                OptionAll::Some(v)
            }
        };
        Ok(id)
    }

    pub fn set_indexed(&mut self, name: &str) -> SResult<(FieldId, IndexedPos)> {
        let id = self.fields_map.insert(name)?;

        if let OptionAll::Some(ref mut v) = self.indexed {
            v.insert(id);
        }

        if let Some(indexed_pos) = self.fields_position.get(&id) {
            return Ok((id, *indexed_pos))
        };
        let pos = self.fields_position.len() as u16;
        self.fields_position.insert(id, pos.into());
        Ok((id, pos.into()))
    }

    pub fn clear_ranked(&mut self) {
        self.ranked.clear();
    }

    pub fn remove_ranked(&mut self, name: &str) {
        if let Some(id) = self.fields_map.id(name) {
            self.ranked.remove(&id);
        }
    }

    pub fn is_ranked(&self, id: FieldId) -> bool {
        self.ranked.get(&id).is_some()
    }

    pub fn is_displayed(&self, id: FieldId) -> bool {
        match self.displayed {
            OptionAll::Some(ref v) => v.contains(&id),
            OptionAll::All => true,
            OptionAll::None => false,
        }
    }

    pub fn is_indexed(&self, id: FieldId) -> Option<&IndexedPos> {
        match self.indexed {
            OptionAll::All => self.fields_position.get(&id),
            OptionAll::Some(ref v) => {
                if v.contains(&id) {
                    self.fields_position.get(&id)
                } else {
                    None
                }
            }
            OptionAll::None => None
        }
    }

    pub fn is_indexed_all(&self) -> bool {
        self.indexed.is_all()
    }

    pub fn indexed_pos_to_field_id<I: Into<IndexedPos>>(&self, pos: I) -> Option<FieldId> {
        let indexed_pos = pos.into().0;
        self
            .fields_position
            .iter()
            .find(|(_, &v)| v.0 == indexed_pos)
            .map(|(&k, _)| k)
    }

    pub fn update_ranked<S: AsRef<str>>(&mut self, data: impl IntoIterator<Item = S>) -> SResult<()> {
        self.ranked.clear();
        for name in data {
            self.set_ranked(name.as_ref())?;
        }
        Ok(())
    }

    pub fn update_displayed<S: AsRef<str>>(&mut self, data: impl IntoIterator<Item = S>) -> SResult<()> {
        self.displayed = match self.displayed.take() {
            OptionAll::Some(mut v) => {
                v.clear();
                OptionAll::Some(v)
            }
            _ => OptionAll::Some(HashSet::new())
        };
        for name in data {
            self.set_displayed(name.as_ref())?;
        }
        Ok(())
    }

    pub fn update_indexed<S: AsRef<str>>(&mut self, data: Vec<S>) -> SResult<()> {
        self.indexed = match self.indexed.take() {
            OptionAll::Some(mut v) => {
                v.clear();
                OptionAll::Some(v)
            },
            _ => OptionAll::Some(HashSet::new()),
        };
        for name in data {
            self.set_indexed(name.as_ref())?;
        }
        Ok(())
    }

    pub fn set_all_fields_as_indexed(&mut self) {
        self.indexed = OptionAll::All;
        self.fields_position.clear();

        for (_name, id) in self.fields_map.iter() {
            let pos = self.fields_position.len() as u16;
            self.fields_position.insert(*id, pos.into());
        }
    }

    pub fn set_all_fields_as_displayed(&mut self) {
        self.displayed = OptionAll::All
    }
}

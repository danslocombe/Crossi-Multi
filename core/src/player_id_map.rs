use serde::{Deserialize, Serialize};

use crate::PlayerId;

// PlayerIdMap is a struct for storing data for each player on the server.
// PlayerIds are a wrapper around u8s assigned in ascending order. 
// A fast and small way to represent the map from PlayerId -> T is via a Vec<Option<T>>.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerIdMap<T> {
    inner : Vec<Option<T>>,
}

impl<T> Default for PlayerIdMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PlayerIdMap<T> {
    pub fn new() -> Self {
        PlayerIdMap {
            inner : Vec::with_capacity(8),
        }
    }

    pub fn from_definition(definition : Vec<(PlayerId, T)>) -> Self {
        let mut vec = Self::new();
        for (id, x) in definition {
            vec.set(id, x);
        }

        vec
    }

    pub fn set(&mut self, id: PlayerId, x : T) {
        //assert!(id.0 != 0);
        let index = id.0 as usize;
        while (index >= self.inner.len())
        {
            self.inner.push(None);
        }

        self.inner[index] = Some(x);
    }

    pub fn remove(&mut self, id : PlayerId) {
        //assert!(id.0 != 0);
        let index = id.0 as usize;
        if index >= self.inner.len() {
            // Nothing to do
        }
        else {
            self.inner[index] = None;
        }
    }

    pub fn get_mut(&mut self, id: crate::PlayerId) -> Option<&mut T> {
        //assert!(id.0 != 0);
        let index = id.0 as usize;
        if index < self.inner.len()
        {
            self.inner[index].as_mut()
        }
        else
        {
            None
        }
    }

    pub fn get(&self, id: crate::PlayerId) -> Option<&T> {
        //assert!(id.0 != 0);
        let index = id.0 as usize;
        if index < self.inner.len()
        {
            self.inner[index].as_ref()
        }
        else
        {
            None
        }
    }

    pub fn contains(&self, id: crate::PlayerId) -> bool {
        self.get(id).is_some()
    }

    pub fn valid_ids(&self) -> Vec<PlayerId> {
        let mut vec = Vec::with_capacity(self.inner.len());
        for i in 0..self.inner.len() {
            if self.inner[i].is_some() {
                vec.push(PlayerId(i as u8));
            }
        }
        vec
    }

    pub fn count_populated(&self) -> usize {
        self.inner.iter().flatten().count()
    }

    pub fn iter(&self) -> PlayerIdMapIterator<'_, T> {
        self.into_iter()
    }

    pub fn next_free(&self) -> Option<PlayerId> {
        for i in 1..8 {
            if i < self.inner.len() {
                if self.inner[i].is_none() {
                    return Some(PlayerId(i as u8));
                }
            }
            else {
                return Some(PlayerId(i as u8));
            }
        }

        None
    }
}

impl<T> PlayerIdMap<T> where T : Clone {
    pub fn get_populated(&self) -> Vec<T> {
        self.inner.iter().flatten().cloned().collect()
    }

    // Create a new map, using the keys from another and a default value
    pub fn seed_from<U>(other : &PlayerIdMap<U>, x : T) -> PlayerIdMap<T> {
        let mut inner = Vec::with_capacity(other.inner.len());
        for other_value in &other.inner {
            match other_value {
                Some(_) => {
                    inner.push(Some(x.clone()));
                }
                None => {
                    inner.push(None);
                }
            }
        }

        PlayerIdMap {
            inner,
        }
    }

    // Given a map and another map, add a default value for keys contained in other map but not this
    pub fn seed_missing<U>(&mut self, other : &PlayerIdMap<U>, x : T) {
        for (id, _) in other.iter() {
            if !self.contains(id) {
                self.set(id, x.clone());
            }
        }
    }

    pub fn intersect<U>(&mut self, other : &PlayerIdMap<U>) {
        for i in 0..self.inner.len() {
            if (!other.contains(PlayerId(i as u8))) {
                self.inner[i] = None;
            }
        }
    }
}

impl<T> PlayerIdMap<T> where T : Copy {
    pub fn get_copy(&self, id: crate::PlayerId) -> Option<T> {
        let index = id.0 as usize;
        if index < self.inner.len()
        {
            self.inner[index]
        }
        else
        {
            None
        }
    }
}

pub struct PlayerIdMapIterator<'a, T>
{
    i : usize,
    vec : &'a PlayerIdMap<T>,
}

impl<'a, T> Iterator for PlayerIdMapIterator<'a, T> {
    type Item = (PlayerId, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        while (self.i < self.vec.inner.len())
        {
            let cur = self.i;
            self.i+=1;

            if let Some(x) = self.vec.inner[cur].as_ref() {
                return Some((PlayerId(cur as u8), x));
            }
        }

        None
    }
}

impl<'a, T> IntoIterator for &'a PlayerIdMap<T> {
    type Item = (PlayerId, &'a T);
    type IntoIter = PlayerIdMapIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        PlayerIdMapIterator {
            i : 0,
            vec : self,
        }
    }
}

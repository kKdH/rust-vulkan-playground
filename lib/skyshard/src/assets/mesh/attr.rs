use std::fmt::Debug;
use std::hash::Hash;
use bimap::BiHashMap;
use crate::assets::mesh::Index;

pub struct AttributeSet<A> {
    attributes: BiHashMap<A, Index>
}

impl <A> AttributeSet<A>
where A: Eq + Hash + Debug {

    pub fn new() -> AttributeSet<A> {
        AttributeSet {
            attributes: BiHashMap::new()
        }
    }

    pub fn insert(&mut self, attribute: A) -> Index {
        match self.attributes.get_by_left(&attribute) {
            None => {
                let index = self.attributes.len() as u32;
                self.attributes.insert(attribute, index);
                index
            }
            Some(index) => *index
        }
    }

    pub fn get(&self, index: Index) -> Option<&A> {
        self.attributes.get_by_right(&index)
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod AttributeSetSpec {
    use crate::assets::mesh::{Coordinate};
    use crate::assets::mesh::attr::AttributeSet;

    #[test]
    fn should_should_return_the_same_index_for_the_same_attribute() {

        let mut set = AttributeSet::<Coordinate>::new();

        assert_eq!(set.insert(Coordinate::new(1.0, 1.0, -1.0)), 0);
        assert_eq!(set.insert(Coordinate::new(-1.0, -1.0, -1.0)), 1);
        assert_eq!(set.insert(Coordinate::new(1.0, 1.0, -1.0)), 0);
    }

    #[test]
    fn should_return_the_attribute_with_the_given_index() {

        let mut set = AttributeSet::<Coordinate>::new();

        let c0 = Coordinate::new(1.0, 1.0, -1.0);
        let c1 = Coordinate::new(-1.0, -1.0, -1.0);

        set.insert(c0.clone());
        set.insert(c1.clone());

        assert_eq!(set.get(0), Some(&c0));
        assert_eq!(set.get(1), Some(&c1));
    }
}

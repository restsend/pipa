use crate::runtime::atom::Atom;
use crate::util::FxHashMap;
use std::cell::{Cell, UnsafeCell};
use std::ptr::NonNull;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShapeId(pub usize);

impl ShapeId {
    pub fn root() -> Self {
        ShapeId(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Transition {
    pub property: Atom,
    pub offset: u32,
}

pub struct Shape {
    pub id: ShapeId,
    pub parent: Option<NonNull<Shape>>,
    pub transition: Option<Transition>,
    pub property_count: u32,
    property_map: UnsafeCell<FxHashMap<Atom, u32>>,
    property_map_built: UnsafeCell<bool>,

    children: UnsafeCell<Vec<(Atom, NonNull<Shape>)>>,
}

unsafe impl Send for Shape {}
unsafe impl Sync for Shape {}

impl Shape {
    pub fn root() -> Self {
        Shape {
            id: ShapeId::root(),
            parent: None,
            transition: None,
            property_count: 0,
            property_map: UnsafeCell::new(FxHashMap::default()),
            property_map_built: UnsafeCell::new(true),
            children: UnsafeCell::new(Vec::new()),
        }
    }

    pub fn add_property(&self, property: Atom, id: ShapeId) -> Shape {
        let offset = self.property_count;
        let parent_ptr = NonNull::from(self);
        Shape {
            id,
            parent: Some(parent_ptr),
            transition: Some(Transition { property, offset }),
            property_count: self.property_count + 1,
            property_map: UnsafeCell::new(FxHashMap::default()),
            property_map_built: UnsafeCell::new(false),
            children: UnsafeCell::new(Vec::new()),
        }
    }

    pub fn get_offset(&self, property: Atom) -> Option<u32> {
        if self.property_count <= 8 {
            unsafe {
                let mut current = Some(NonNull::from(self));
                while let Some(shape_ptr) = current {
                    let shape = &*shape_ptr.as_ptr();
                    if let Some(ref t) = shape.transition {
                        if t.property == property {
                            return Some(t.offset);
                        }
                    }
                    current = shape.parent;
                }
            }
            return None;
        }
        self.ensure_property_map();

        unsafe { (*self.property_map.get()).get(&property).copied() }
    }

    fn ensure_property_map(&self) {
        unsafe {
            if *self.property_map_built.get() {
                return;
            }

            let map = &mut *self.property_map.get();
            map.reserve(self.property_count as usize);
            let mut current = Some(NonNull::from(self));

            while let Some(shape_ptr) = current {
                let shape = &*shape_ptr.as_ptr();
                if let Some(transition) = &shape.transition {
                    map.insert(transition.property, transition.offset);
                }
                current = shape.parent;
            }

            *self.property_map_built.get() = true;
        }
    }

    pub fn properties(&self) -> Vec<(Atom, u32)> {
        let mut props = Vec::with_capacity(self.property_count as usize);
        let mut current = Some(NonNull::from(self));

        while let Some(shape_ptr) = current {
            unsafe {
                let shape = &*shape_ptr.as_ptr();
                if let Some(transition) = &shape.transition {
                    props.push((transition.property, transition.offset));
                }
                current = shape.parent;
            }
        }

        props.reverse();
        props
    }

    pub fn has_property(&self, property: Atom) -> bool {
        self.get_offset(property).is_some()
    }

    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }
}

impl std::fmt::Debug for Shape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shape")
            .field("id", &self.id)
            .field("property_count", &self.property_count)
            .field("transition", &self.transition)
            .finish()
    }
}

pub struct ShapeCache {
    root_shape: NonNull<Shape>,
    next_id: Cell<usize>,
    shapes: Vec<Box<Shape>>,
}

impl ShapeCache {
    pub fn new() -> Self {
        let root = Box::new(Shape::root());
        let root_ptr = NonNull::from(&*root);

        ShapeCache {
            root_shape: root_ptr,
            next_id: Cell::new(1),
            shapes: vec![root],
        }
    }

    pub fn alloc_id(&self) -> ShapeId {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        ShapeId(id)
    }

    pub fn root_shape(&self) -> NonNull<Shape> {
        self.root_shape
    }

    pub fn transition(&mut self, parent: NonNull<Shape>, property: Atom) -> NonNull<Shape> {
        unsafe {
            let parent_ref = &*parent.as_ptr();

            let children = &mut *parent_ref.children.get();
            for &(atom, child_ptr) in children.iter() {
                if atom == property {
                    return child_ptr;
                }
            }

            let child_id = self.alloc_id();
            let child = Box::new(parent_ref.add_property(property, child_id));
            let child_ptr = NonNull::from(&*child);
            children.push((property, child_ptr));
            self.shapes.push(child);
            child_ptr
        }
    }

    pub fn create_shape_for_properties(&mut self, properties: &[Atom]) -> NonNull<Shape> {
        let mut current = self.root_shape;

        for property in properties {
            current = self.transition(current, *property);
        }

        current
    }
}

impl Default for ShapeCache {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ShapeCache {
    fn drop(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::atom::AtomTable;

    #[test]
    fn test_root_shape() {
        let shape = Shape::root();
        assert!(shape.is_root());
        assert_eq!(shape.property_count, 0);
    }

    #[test]
    fn test_shape_transitions() {
        let mut atoms = AtomTable::new();
        let foo = atoms.intern("foo");
        let bar = atoms.intern("bar");

        let mut cache = ShapeCache::new();
        let root = cache.root_shape();

        let shape1 = cache.transition(root, foo);
        unsafe {
            let s1 = &*shape1.as_ptr();
            assert_eq!(s1.property_count, 1);
            assert_eq!(s1.get_offset(foo), Some(0));
            assert_eq!(s1.get_offset(bar), None);
        }

        let shape2 = cache.transition(shape1, bar);
        unsafe {
            let s2 = &*shape2.as_ptr();
            assert_eq!(s2.property_count, 2);
            assert_eq!(s2.get_offset(foo), Some(0));
            assert_eq!(s2.get_offset(bar), Some(1));
        }
    }

    #[test]
    fn test_shape_caching() {
        let mut atoms = AtomTable::new();
        let foo = atoms.intern("foo");

        let mut cache = ShapeCache::new();
        let root = cache.root_shape();

        let shape1 = cache.transition(root, foo);
        let shape2 = cache.transition(root, foo);

        assert_eq!(shape1, shape2);
    }

    #[test]
    fn test_properties_iteration() {
        let mut atoms = AtomTable::new();
        let foo = atoms.intern("foo");
        let bar = atoms.intern("bar");
        let baz = atoms.intern("baz");

        let mut cache = ShapeCache::new();
        let root = cache.root_shape();

        let shape1 = cache.transition(root, foo);
        let shape2 = cache.transition(shape1, bar);
        let shape3 = cache.transition(shape2, baz);

        unsafe {
            let s3 = &*shape3.as_ptr();
            let props = s3.properties();
            assert_eq!(props.len(), 3);
            assert_eq!(props[0], (foo, 0));
            assert_eq!(props[1], (bar, 1));
            assert_eq!(props[2], (baz, 2));
        }
    }
}

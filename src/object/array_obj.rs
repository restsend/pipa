use crate::object::object::JSObject;
use crate::value::JSValue;

#[repr(C)]
pub struct JSArrayObject {
    pub header: JSObject,
    pub elements: Vec<JSValue>,
}

impl JSArrayObject {
    pub fn new() -> Self {
        let mut header = JSObject::new_array();
        header.set_dense_array_flag();
        JSArrayObject {
            header,
            elements: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let mut header = JSObject::new_array();
        header.set_dense_array_flag();
        JSArrayObject {
            header,
            elements: Vec::with_capacity(capacity),
        }
    }

    pub fn with_length(length: usize) -> Self {
        let mut header = JSObject::new_array();
        header.set_dense_array_flag();
        let elements = if length <= 100_000 {
            vec![JSValue::undefined(); length]
        } else {
            Vec::with_capacity(length)
        };
        JSArrayObject { header, elements }
    }

    pub fn from_elements(elements: Vec<JSValue>) -> Self {
        let mut header = JSObject::new_array();
        header.set_dense_array_flag();
        JSArrayObject { header, elements }
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<JSValue> {
        self.elements.get(index).copied()
    }

    #[inline]
    pub fn set(&mut self, index: usize, value: JSValue) {
        if index < self.elements.len() {
            self.elements[index] = value;
        } else if index == self.elements.len() && index < 100000 {
            self.elements.push(value);
        } else if index < 100000 {
            self.elements.resize(index + 1, JSValue::undefined());
            self.elements[index] = value;
        }
    }

    #[inline]
    pub fn push(&mut self, value: JSValue) {
        self.elements.push(value);
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn for_each_element(&self, mut f: impl FnMut(&JSValue)) {
        for value in self.elements.iter() {
            f(value);
        }
    }

    pub fn set_elements(&mut self, elements: Vec<JSValue>) {
        self.elements = elements;
    }

    #[inline]
    pub fn elements(&self) -> &Vec<JSValue> {
        &self.elements
    }

    #[inline]
    pub fn elements_mut(&mut self) -> &mut Vec<JSValue> {
        &mut self.elements
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::JSValue;

    #[test]
    fn test_array_object_new() {
        let arr = JSArrayObject::new();
        assert!(arr.is_empty());
        assert_eq!(arr.len(), 0);
        assert!(arr.header.is_array());
    }

    #[test]
    fn test_array_object_push_and_get() {
        let mut arr = JSArrayObject::new();
        arr.push(JSValue::new_int(1));
        arr.push(JSValue::new_int(2));
        arr.push(JSValue::new_int(3));

        assert_eq!(arr.len(), 3);
        assert_eq!(arr.get(0).unwrap().get_int(), 1);
        assert_eq!(arr.get(1).unwrap().get_int(), 2);
        assert_eq!(arr.get(2).unwrap().get_int(), 3);
        assert!(arr.get(3).is_none());
    }

    #[test]
    fn test_array_object_set_existing() {
        let mut arr = JSArrayObject::from_elements(vec![JSValue::new_int(0); 5]);
        arr.set(2, JSValue::new_int(99));
        assert_eq!(arr.get(2).unwrap().get_int(), 99);
        assert_eq!(arr.len(), 5);
    }

    #[test]
    fn test_array_object_set_extends() {
        let mut arr = JSArrayObject::new();
        arr.set(3, JSValue::new_int(42));
        assert_eq!(arr.len(), 4);
        assert_eq!(arr.get(3).unwrap().get_int(), 42);

        assert!(arr.get(0).unwrap().is_undefined());
    }

    #[test]
    fn test_array_object_from_elements() {
        let arr = JSArrayObject::from_elements(vec![JSValue::new_int(10), JSValue::new_int(20)]);
        assert_eq!(arr.len(), 2);
        assert_eq!(arr.get(0).unwrap().get_int(), 10);
    }

    #[test]
    fn test_array_object_set_elements() {
        let mut arr = JSArrayObject::new();
        arr.push(JSValue::new_int(1));
        arr.set_elements(vec![JSValue::new_int(99)]);
        assert_eq!(arr.len(), 1);
        assert_eq!(arr.get(0).unwrap().get_int(), 99);
    }

    #[test]
    fn test_array_object_for_each_element() {
        let mut arr = JSArrayObject::new();
        arr.push(JSValue::new_int(10));
        arr.push(JSValue::new_int(20));

        let mut sum = 0i64;
        arr.for_each_element(|v| {
            sum += v.get_int();
        });
        assert_eq!(sum, 30);
    }

    #[test]
    fn test_array_object_with_capacity() {
        let arr = JSArrayObject::with_capacity(100);
        assert_eq!(arr.len(), 0);
        assert!(arr.elements().capacity() >= 100);
    }

    #[test]
    fn test_array_object_elements_accessor() {
        let mut arr = JSArrayObject::new();
        arr.push(JSValue::new_int(1));
        assert_eq!(arr.elements().len(), 1);
        arr.elements_mut().push(JSValue::new_int(2));
        assert_eq!(arr.len(), 2);
    }
}

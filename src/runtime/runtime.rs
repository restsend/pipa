use super::atom::AtomTable;
use super::context::JSContext;
use super::gc::GcHeap;
use super::module::ModuleRegistry;
#[cfg(feature = "fetch")]
use crate::http::conn::Connection;
use crate::value::JSValue;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct JSRuntime {
    atom_table: AtomTable,
    gc_heap: GcHeap,
    ctx_count: u32,
    interrupt: AtomicBool,
    module_registry: ModuleRegistry,
    next_symbol_id: u32,
    
    pub argv: Vec<String>,
    
    #[cfg(feature = "fetch")]
    pub connections: Vec<Option<Box<Connection>>>,
}

impl JSRuntime {
    pub fn new() -> Self {
        JSRuntime {
            atom_table: AtomTable::new(),
            gc_heap: GcHeap::new(),
            ctx_count: 0,
            interrupt: AtomicBool::new(false),
            module_registry: ModuleRegistry::new(),
            next_symbol_id: 1,
            argv: Vec::new(),
            #[cfg(feature = "fetch")]
            connections: Vec::new(),
        }
    }

    pub fn set_interrupt(&self, flag: bool) {
        self.interrupt.store(flag, Ordering::SeqCst);
    }

    pub fn is_interrupted(&self) -> bool {
        self.interrupt.load(Ordering::SeqCst)
    }

    pub fn atom_table(&self) -> &AtomTable {
        &self.atom_table
    }

    pub fn atom_table_mut(&mut self) -> &mut AtomTable {
        &mut self.atom_table
    }

    pub fn gc_heap(&self) -> &GcHeap {
        &self.gc_heap
    }

    pub fn gc_heap_mut(&mut self) -> &mut GcHeap {
        &mut self.gc_heap
    }

    pub fn module_registry(&self) -> &ModuleRegistry {
        &self.module_registry
    }

    pub fn module_registry_mut(&mut self) -> &mut ModuleRegistry {
        &mut self.module_registry
    }

    pub fn new_context(&mut self) -> JSContext {
        self.ctx_count += 1;
        JSContext::new(self)
    }

    pub fn run_gc(&mut self, roots: &[JSValue]) -> usize {
        self.gc_heap.run_gc(roots)
    }

    pub fn minor_gc(&mut self, roots: &[JSValue]) -> usize {
        self.gc_heap.minor_gc(roots)
    }

    pub fn atom_count(&self) -> usize {
        self.atom_table.len()
    }

    pub fn context_count(&self) -> u32 {
        self.ctx_count
    }

    pub fn next_symbol_id(&mut self) -> u32 {
        let id = self.next_symbol_id;
        self.next_symbol_id += 1;
        id
    }

    pub fn set_argv(&mut self, args: Vec<String>) {
        self.argv = args;
    }

    #[cfg(feature = "fetch")]
    pub fn add_connection(&mut self, conn: Connection) -> usize {
        let idx = self.connections.len();
        self.connections.push(Some(Box::new(conn)));
        idx
    }

    #[cfg(feature = "fetch")]
    pub fn release_connection(&mut self, idx: usize) {
        if let Some(slot) = self.connections.get_mut(idx) {
            *slot = None;
        }
    }

    #[cfg(feature = "fetch")]
    pub fn get_connection(&mut self, idx: usize) -> Option<&mut Connection> {
        self.connections
            .get_mut(idx)
            .and_then(|slot| slot.as_mut().map(|b| b.as_mut()))
    }
}

impl Default for JSRuntime {
    fn default() -> Self {
        Self::new()
    }
}

use std::collections::HashMap;
use std::os::unix::io::RawFd;

#[cfg(feature = "fetch")]
use crate::http::request::{HttpRequest, RequestEvent};
#[cfg(feature = "fetch")]
use crate::http::ws::conn::{WsConnection, WsEvent};
use crate::util::iomux::Poller;

use super::context::JSContext;
use super::extension::MacroTaskExtension;

pub enum ReactorTask {
    #[cfg(feature = "fetch")]
    Http(HttpRequest),
    #[cfg(feature = "fetch")]
    Ws(WsConnection),
}

pub enum AdvanceResult {
    None,
    #[cfg(feature = "fetch")]
    HttpDone(HttpRequest, RequestEvent),
    #[cfg(feature = "fetch")]
    WsEvent(WsEvent),
}

impl ReactorTask {
    pub fn fd(&self) -> Option<RawFd> {
        match self {
            #[cfg(feature = "fetch")]
            ReactorTask::Http(r) => r.fd(),
            #[cfg(feature = "fetch")]
            ReactorTask::Ws(w) => w.fd(),
        }
    }

    pub fn wants_read(&self) -> bool {
        match self {
            #[cfg(feature = "fetch")]
            ReactorTask::Http(r) => r.wants_read(),
            _ => true,
        }
    }

    pub fn wants_write(&self) -> bool {
        match self {
            #[cfg(feature = "fetch")]
            ReactorTask::Http(r) => r.wants_write(),
            _ => false,
        }
    }

    pub fn task_type(&self) -> &str {
        match self {
            #[cfg(feature = "fetch")]
            ReactorTask::Http(_) => "http",
            #[cfg(feature = "fetch")]
            ReactorTask::Ws(_) => "ws",
        }
    }
}

pub struct IoReactor {
    poller: Poller,
    tasks: HashMap<u64, ReactorTask>,
    next_id: u64,
}

impl IoReactor {
    pub fn new() -> Result<Self, String> {
        let poller = Poller::new()?;
        Ok(IoReactor {
            poller,
            tasks: HashMap::new(),
            next_id: 1,
        })
    }

    pub fn register(&mut self, fd: RawFd, task: ReactorTask) -> Result<u64, String> {
        let id = self.next_id;
        self.next_id += 1;

        self.poller
            .register(fd, task.wants_read(), task.wants_write())?;

        self.tasks.insert(id, task);
        Ok(id)
    }

    fn update_registration(&mut self, id: u64) -> Result<(), String> {
        let task = self
            .tasks
            .get(&id)
            .ok_or_else(|| format!("task {id} not found"))?;

        if let Some(fd) = task.fd() {
            self.poller
                .modify(fd, task.wants_read(), task.wants_write())?;
        }
        Ok(())
    }

    fn unregister(&mut self, id: u64) -> Option<ReactorTask> {
        let task = self.tasks.remove(&id);
        if let Some(ref t) = task {
            if let Some(fd) = t.fd() {
                let _ = self.poller.unregister(fd);
            }
        }
        task
    }

    #[cfg(feature = "fetch")]
    pub fn get_http_mut(&mut self, id: u64) -> Option<&mut HttpRequest> {
        match self.tasks.get_mut(&id) {
            Some(ReactorTask::Http(r)) => Some(r),
            _ => None,
        }
    }

    #[cfg(feature = "fetch")]
    pub fn get_ws_mut(&mut self, id: u64) -> Option<&mut WsConnection> {
        match self.tasks.get_mut(&id) {
            Some(ReactorTask::Ws(w)) => Some(w),
            _ => None,
        }
    }

    fn poll(&mut self, timeout_ms: i32) -> Result<Vec<(u64, String, PollEvent)>, String> {
        if self.tasks.is_empty() {
            return Ok(Vec::new());
        }

        let events = self.poller.wait(timeout_ms)?;
        let mut results = Vec::new();

        for event in &events {
            let ids_to_check: Vec<u64> = self
                .tasks
                .iter()
                .filter(|(_, t)| t.fd() == Some(event.fd))
                .map(|(id, _)| *id)
                .collect();

            for id in ids_to_check {
                let task_type = self.tasks.get(&id).map(|t| t.task_type().to_string());
                if event.error {
                    results.push((id, task_type.unwrap_or_default(), PollEvent::Error));
                } else if event.readable {
                    results.push((id, task_type.unwrap_or_default(), PollEvent::Readable));
                } else if event.writable {
                    results.push((id, task_type.unwrap_or_default(), PollEvent::Writable));
                }
            }
        }

        Ok(results)
    }

    #[cfg(feature = "fetch")]
    fn advance_http(&mut self, id: u64) -> Result<AdvanceResult, String> {
        let task = self
            .tasks
            .get_mut(&id)
            .ok_or_else(|| format!("http task {id} not found"))?;
        match task {
            ReactorTask::Http(req) => {
                let event = req.try_advance()?;
                match &event {
                    RequestEvent::NeedRead | RequestEvent::NeedWrite => {
                        self.update_registration(id)?;
                    }
                    RequestEvent::Complete(_) | RequestEvent::Error(_) => {
                        let mut taken = HttpRequest::dummy();
                        std::mem::swap(req, &mut taken);
                        self.unregister(id);
                        return Ok(AdvanceResult::HttpDone(taken, event));
                    }
                }
                Ok(AdvanceResult::None)
            }
            ReactorTask::Ws(_) => Err(format!("task {id} is not http")),
        }
    }

    #[cfg(feature = "fetch")]
    fn advance_ws(&mut self, id: u64) -> Result<AdvanceResult, String> {
        let task = self
            .tasks
            .get_mut(&id)
            .ok_or_else(|| format!("ws task {id} not found"))?;
        match task {
            ReactorTask::Ws(ws) => {
                let event = ws.try_advance()?;
                match &event {
                    Some(ev) => {
                        if matches!(ev, WsEvent::Close(_, _) | WsEvent::Error(_)) {
                            self.unregister(id);
                        }
                        Ok(AdvanceResult::WsEvent(ev.clone()))
                    }
                    None => {
                        self.update_registration(id)?;
                        Ok(AdvanceResult::None)
                    }
                }
            }
            ReactorTask::Http(_) => Err(format!("task {id} is not ws")),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

impl MacroTaskExtension for IoReactor {
    fn tick(&mut self, _ctx: &mut JSContext) -> Result<bool, String> {
        if self.is_empty() {
            return Ok(false);
        }

        let events = self.poll(0)?;
        for (id, task_type, _) in &events {
            match task_type.as_str() {
                #[cfg(feature = "fetch")]
                "http" => {
                    let _ = self.advance_http(*id);
                    let _ = self.update_registration(*id);
                }
                #[cfg(feature = "fetch")]
                "ws" => {
                    let _ = self.advance_ws(*id);
                    let _ = self.update_registration(*id);
                }
                _ => {}
            }
        }

        Ok(!events.is_empty())
    }

    fn has_pending(&self) -> bool {
        !self.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollEvent {
    Readable,
    Writable,
    Error,
}

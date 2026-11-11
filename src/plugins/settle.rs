//! How a plugin entry ends: pure state (no Leptos, no DOM), tested on the host.
//!
//! A [`Settlement`] is either *pending* (it collects listeners) or *settled* with a
//! `Result<T, Dismissed>`. The first settle wins; a listener added after that fires right away.
//! Every mutating method returns a [`Flush`] instead of calling the listeners itself: the caller
//! runs it **after** releasing whatever holds the settlement (a `StoredValue`), so a listener
//! can safely call back into the handle that owns it.

use std::fmt;

/// Why a plugin entry ended without a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dismissed {
    /// The view asked for it (`PluginCtx::cancel`, e.g. a "Cancel" button).
    Cancel,
    /// The user closed it with `Esc` or a click outside, not through the plugin API.
    User,
    /// Code closed it: `PluginHandle::hide`, `PluginCtx::hide` or `Plugins::dismiss_all`.
    Programmatic,
    /// It never opened (no host, no context, running on the server) or the host went away.
    Unavailable,
}

impl fmt::Display for Dismissed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Dismissed::Cancel => "cancelled by the view",
            Dismissed::User => "dismissed by the user",
            Dismissed::Programmatic => "hidden by code",
            Dismissed::Unavailable => "plugin host unavailable",
        })
    }
}

impl std::error::Error for Dismissed {}

/// Listeners ready to run, taken out of the [`Settlement`].
#[must_use = "listeners only run when the flush is run"]
pub(crate) struct Flush(Vec<Box<dyn FnOnce()>>);

impl Flush {
    fn none() -> Self {
        Flush(Vec::new())
    }

    /// Runs every listener once, in registration order.
    pub(crate) fn run(self) {
        for f in self.0 {
            f();
        }
    }
}

type ResultListener<T> = Box<dyn FnOnce(Result<T, Dismissed>)>;

/// Pending listeners or the settled result of one plugin entry.
pub(crate) struct Settlement<T> {
    settled: Option<Result<T, Dismissed>>,
    on_ok: Vec<Box<dyn FnOnce(T)>>,
    on_cancel: Vec<Box<dyn FnOnce(Dismissed)>>,
    /// Run after `ok`/`cancel` listeners: dismiss listeners and future waiters.
    on_any: Vec<ResultListener<T>>,
}

impl<T: Clone + 'static> Settlement<T> {
    pub(crate) fn new() -> Self {
        Settlement {
            settled: None,
            on_ok: Vec::new(),
            on_cancel: Vec::new(),
            on_any: Vec::new(),
        }
    }

    /// The settled result, if any.
    pub(crate) fn result(&self) -> Option<Result<T, Dismissed>> {
        self.settled.clone()
    }

    /// Settles with `result` unless already settled (the first settle wins). The pending
    /// listeners are taken out here; they run when the returned [`Flush`] does.
    pub(crate) fn settle(&mut self, result: Result<T, Dismissed>) -> Flush {
        if self.settled.is_some() {
            return Flush::none();
        }
        self.settled = Some(result.clone());
        let mut jobs: Vec<Box<dyn FnOnce()>> = Vec::new();
        match &result {
            Ok(value) => {
                for f in std::mem::take(&mut self.on_ok) {
                    let value = value.clone();
                    jobs.push(Box::new(move || f(value)));
                }
                self.on_cancel.clear();
            }
            Err(why) => {
                for f in std::mem::take(&mut self.on_cancel) {
                    let why = *why;
                    jobs.push(Box::new(move || f(why)));
                }
                self.on_ok.clear();
            }
        }
        for f in std::mem::take(&mut self.on_any) {
            let result = result.clone();
            jobs.push(Box::new(move || f(result)));
        }
        Flush(jobs)
    }

    /// Calls `f` with the value when (or if already) settled with `Ok`.
    pub(crate) fn on_ok(&mut self, f: impl FnOnce(T) + 'static) -> Flush {
        match &self.settled {
            None => {
                self.on_ok.push(Box::new(f));
                Flush::none()
            }
            Some(Ok(value)) => {
                let value = value.clone();
                Flush(vec![Box::new(move || f(value))])
            }
            Some(Err(_)) => Flush::none(),
        }
    }

    /// Calls `f` with the reason when (or if already) settled with `Err`.
    pub(crate) fn on_cancel(&mut self, f: impl FnOnce(Dismissed) + 'static) -> Flush {
        match &self.settled {
            None => {
                self.on_cancel.push(Box::new(f));
                Flush::none()
            }
            Some(Err(why)) => {
                let why = *why;
                Flush(vec![Box::new(move || f(why))])
            }
            Some(Ok(_)) => Flush::none(),
        }
    }

    /// Calls `f` with the result when (or if already) settled, whichever way.
    pub(crate) fn wait(&mut self, f: impl FnOnce(Result<T, Dismissed>) + 'static) -> Flush {
        match &self.settled {
            None => {
                self.on_any.push(Box::new(f));
                Flush::none()
            }
            Some(result) => {
                let result = result.clone();
                Flush(vec![Box::new(move || f(result))])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use super::*;

    type Log = Rc<RefCell<Vec<String>>>;

    fn log() -> Log {
        Rc::new(RefCell::new(Vec::new()))
    }

    #[test]
    fn ok_runs_ok_then_dismiss_listeners_and_skips_cancel() {
        let log = log();
        let mut s = Settlement::<u32>::new();
        let l = log.clone();
        s.on_ok(move |v| l.borrow_mut().push(format!("ok {v}")))
            .run();
        let l = log.clone();
        s.on_cancel(move |w| l.borrow_mut().push(format!("cancel {w:?}")))
            .run();
        let l = log.clone();
        s.wait(move |r| l.borrow_mut().push(format!("any {r:?}")))
            .run();
        s.settle(Ok(7)).run();
        assert_eq!(*log.borrow(), vec!["ok 7", "any Ok(7)"]);
    }

    #[test]
    fn err_runs_cancel_and_any_listeners_and_skips_ok() {
        let log = log();
        let mut s = Settlement::<u32>::new();
        let l = log.clone();
        s.on_ok(move |v| l.borrow_mut().push(format!("ok {v}")))
            .run();
        let l = log.clone();
        s.on_cancel(move |w| l.borrow_mut().push(format!("cancel {w:?}")))
            .run();
        let l = log.clone();
        s.wait(move |r| l.borrow_mut().push(format!("any {r:?}")))
            .run();
        s.settle(Err(Dismissed::User)).run();
        assert_eq!(*log.borrow(), vec!["cancel User", "any Err(User)"]);
    }

    #[test]
    fn the_first_settle_wins() {
        let mut s = Settlement::<u32>::new();
        s.settle(Ok(1)).run();
        s.settle(Err(Dismissed::Cancel)).run();
        s.settle(Ok(2)).run();
        assert_eq!(s.result(), Some(Ok(1)));
    }

    #[test]
    fn late_listeners_fire_immediately() {
        let log = log();
        let mut s = Settlement::<u32>::new();
        s.settle(Err(Dismissed::Programmatic)).run();
        let l = log.clone();
        s.on_cancel(move |w| l.borrow_mut().push(format!("cancel {w:?}")))
            .run();
        let l = log.clone();
        s.on_ok(move |v| l.borrow_mut().push(format!("ok {v}")))
            .run();
        let l = log.clone();
        s.wait(move |r| l.borrow_mut().push(format!("any {r:?}")))
            .run();
        assert_eq!(
            *log.borrow(),
            vec!["cancel Programmatic", "any Err(Programmatic)"]
        );
    }

    #[test]
    fn listeners_are_taken_out_before_running() {
        let log = log();
        let mut s = Settlement::<u32>::new();
        s.wait(push_result(&log)).run();
        let flush = s.settle(Ok(1));
        // Nothing ran yet and nothing is left in the settlement: settling again does nothing.
        assert!(log.borrow().is_empty());
        s.settle(Ok(2)).run();
        assert!(log.borrow().is_empty());
        flush.run();
        assert_eq!(*log.borrow(), vec!["Ok(1)"]);
        // Each listener ran exactly once.
        s.settle(Ok(3)).run();
        assert_eq!(log.borrow().len(), 1);
    }

    fn push_result(log: &Log) -> impl FnOnce(Result<u32, Dismissed>) + 'static {
        let log = log.clone();
        move |r| log.borrow_mut().push(format!("{r:?}"))
    }

    #[test]
    fn a_dropped_waiter_is_visible_to_a_oneshot_as_canceled() {
        use futures::{channel::oneshot, executor::block_on};
        let (tx, rx) = oneshot::channel::<Result<u32, Dismissed>>();
        let mut s = Settlement::<u32>::new();
        s.wait(move |r| {
            let _ = tx.send(r);
        })
        .run();
        drop(s);
        assert!(block_on(rx).is_err());

        let (tx, rx) = oneshot::channel::<Result<u32, Dismissed>>();
        let mut s = Settlement::<u32>::new();
        s.wait(move |r| {
            let _ = tx.send(r);
        })
        .run();
        s.settle(Ok(5)).run();
        assert_eq!(block_on(rx), Ok(Ok(5)));
    }
}

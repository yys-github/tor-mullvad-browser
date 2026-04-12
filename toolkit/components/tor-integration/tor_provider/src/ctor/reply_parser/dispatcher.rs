// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use bytes::Bytes;
use std::{cell::RefCell, collections::VecDeque, mem, rc::Rc};

use super::{error::ReplyError, factory::ReplyFactory, reply::Reply};

type ReplyCallback = Box<dyn FnOnce(Result<Reply, ReplyError>)>;

/// Parser that accepts bytes from any source, parses them as Tor control-port
/// replies, and calls functions from a queue whenever a reply is ready.
///
/// Order is guaranteed only for replies of the same type.
/// I.e., synchronous replies will be dispatched in order, asynchronous
/// notifications will be dispatched in order, but we might dispatch an
/// asynchronous notification before a synchronous reply we received previously.
///
/// Thread safety: the dispatcher should be always called from the same thread,
/// and will call callbacks from that thread, too.
///
/// Reentrancy safety: it is safe to call public methods of the dispatcher from
/// callbacks.
///
/// Panic safety: panics in callbacks/async handlers will leave the dispatcher
/// in an undefined state.
pub struct ReplyDispatcher {
    callbacks: RefCell<VecDeque<ReplyCallback>>,
    async_handler: RefCell<Option<Rc<dyn Fn(Reply)>>>,
    reply_factory: RefCell<ReplyFactory>,
    /// A queue of synchronous replies that are waiting to be dispatched.
    /// It guarantees that callbacks will be called in order even when
    /// re-entering.
    /// This queue must not contain asynchronous notifications.
    reply_queue: RefCell<VecDeque<Reply>>,
}

impl ReplyDispatcher {
    /// Create a new dispatcher.
    pub fn new() -> Self {
        Self {
            callbacks: RefCell::new(VecDeque::new()),
            async_handler: RefCell::new(None),
            reply_factory: RefCell::new(ReplyFactory::default()),
            reply_queue: RefCell::new(VecDeque::new()),
        }
    }

    /// Register a callback that will be invoked for the next complete reply.
    pub fn push_callback(&self, cb: ReplyCallback) {
        // Reentrancy safety: we do not use the callback yet.
        self.callbacks.borrow_mut().push_back(cb);
        self.maybe_dispatch();
    }

    /// Register the handler for async notifications.
    pub fn set_async_handler(&self, cb: Box<dyn Fn(Reply)>) {
        // Reentrancy safety: we do not use the handler yet.
        *self.async_handler.borrow_mut() = Some(Rc::from(cb))
    }

    /// Feed data to parse.
    /// No action will be taken until a full line is seen.
    ///
    /// When this function returns an error, it automatically propagates it also
    /// to all the callbacks and dequeues them.
    /// The rationale is that we are handling only protocol error, which are
    /// serious errors. Trying to recover from them might make us associate a
    /// reply to a different callback that was not intended for it.
    pub fn feed(&self, data: &Bytes) -> Result<(), ReplyError> {
        // We rely on the factory not being re-entrant, i.e., it will never call
        // us back. This makes the borrow safe from panicking.
        let maybe_new_replies = self.reply_factory.borrow_mut().build(data);
        let mut new_replies = match maybe_new_replies {
            Ok(r) => r,
            Err(e) => {
                self.fail_all(e.clone());
                return Err(e);
            }
        };

        // Reentrancy safety: we sort out replies by type, without calling
        // external functions (except the data structures ones).
        let mut async_replies = Vec::with_capacity(new_replies.len());
        {
            let mut dispatch_queue = self.reply_queue.borrow_mut();
            for reply in new_replies.drain(..) {
                if reply.is_async() {
                    async_replies.push(reply);
                } else {
                    dispatch_queue.push_back(reply);
                }
            }
        }

        // The rest of the function might cause reentrancy!

        for reply in async_replies.drain(..) {
            // We allow the handler to change in case of reentrancy
            let maybe_handler = self.async_handler.borrow().clone();
            match maybe_handler {
                Some(f) => f(reply),
                // However, if we do not have any handler, there is no way we
                // get one now, so drop the existing notifications.
                None => break,
            }
        }

        self.maybe_dispatch();
        Ok(())
    }

    /// Try dispatching the replies, until the reply queue is empty, or until we
    /// empty the queue of registered callbacks for synchronous replies.
    fn maybe_dispatch(&self) {
        // We need to continuously borrow the queues because we cannot we want
        // to ensure the callback propagation order.
        // If we created a local zipped queue now, in case of reentrancy, this
        // guarantee could break.
        while let Some((reply, callback)) = self.pop_reply_callback() {
            debug_assert!(!reply.is_async(), "How did an async reply get here?");
            callback(Ok(reply));
        }
    }

    fn pop_reply_callback(&self) -> Option<(Reply, ReplyCallback)> {
        let mut replies = self.reply_queue.borrow_mut();
        let mut callbacks = self.callbacks.borrow_mut();
        if replies.is_empty() || callbacks.is_empty() {
            return None;
        }
        let pair = replies.pop_front().zip(callbacks.pop_front());
        debug_assert!(
            pair.is_some(),
            "We checked right above, how could this be None?! We might have fallen out of sync at this point."
        );
        pair
    }

    /// Signal a fatal error to all queued callbacks.
    /// The queue will be emptied and any pending reply reset.
    pub fn fail_all(&self, err: ReplyError) {
        let mut callbacks = mem::take(&mut *self.callbacks.borrow_mut());
        self.reply_factory.borrow_mut().clear();
        self.reply_queue.borrow_mut().clear();
        while let Some(cb) = callbacks.pop_front() {
            (cb)(Err(err.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{line::*, test_utils::check_250_ok};
    use super::*;

    use std::cell::Cell;

    #[test]
    fn dispatcher_basic() {
        let dispatcher = ReplyDispatcher::new();
        let called = Rc::new(Cell::new(false));
        let c = called.clone();
        dispatcher.push_callback(Box::new(move |r| {
            check_250_ok(&r.unwrap(), false);
            c.set(true);
        }));
        dispatcher.feed(&Bytes::from_static(b"250 OK\r\n")).unwrap();
        assert!(called.get());
    }

    #[test]
    fn with_details() {
        let dispatcher = ReplyDispatcher::new();
        let called = Rc::new(Cell::new(false));
        let c = called.clone();
        dispatcher.push_callback(Box::new(move |r| {
            check_250_ok(&r.unwrap(), true);
            c.set(true);
        }));
        dispatcher
            .feed(&Bytes::from_static(b"250-test=value\r\n"))
            .unwrap();
        assert!(!called.get());
        dispatcher.feed(&Bytes::from_static(b"250 OK\r\n")).unwrap();
        assert!(called.get());
    }

    #[test]
    fn multiple_commands() {
        let dispatcher = ReplyDispatcher::new();
        let calls = Rc::new(Cell::new(0));
        let c1 = calls.clone();
        dispatcher.push_callback(Box::new(move |r| {
            assert_eq!(
                *r.unwrap().end_line(),
                EndReplyLine {
                    code: 250,
                    line: Bytes::from_static(b"C1")
                }
            );
            c1.set(c1.get() + 1);
        }));
        let c2 = calls.clone();
        dispatcher.push_callback(Box::new(move |r| {
            assert_eq!(
                *r.unwrap().end_line(),
                EndReplyLine {
                    code: 250,
                    line: Bytes::from_static(b"C2")
                }
            );
            c2.set(c2.get() + 1);
        }));
        dispatcher.feed(&Bytes::from_static(b"250 C1\r\n")).unwrap();
        dispatcher.feed(&Bytes::from_static(b"250 C2\r\n")).unwrap();
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn multiple_single_feed() {
        let dispatcher = ReplyDispatcher::new();
        let calls = Rc::new(Cell::new(0));
        let c1 = calls.clone();
        dispatcher.push_callback(Box::new(move |r| {
            assert_eq!(
                *r.unwrap().end_line(),
                EndReplyLine {
                    code: 250,
                    line: Bytes::from_static(b"C1")
                }
            );
            c1.set(c1.get() + 1);
        }));
        let c2 = calls.clone();
        dispatcher.push_callback(Box::new(move |r| {
            assert_eq!(
                *r.unwrap().end_line(),
                EndReplyLine {
                    code: 250,
                    line: Bytes::from_static(b"C2")
                }
            );
            c2.set(c2.get() + 1);
        }));
        dispatcher
            .feed(&Bytes::from_static(b"250 C1\r\n250 C2\r\n"))
            .unwrap();
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn split_feed() {
        let dispatcher = ReplyDispatcher::new();
        let called = Rc::new(Cell::new(false));
        let c2 = called.clone();
        dispatcher.push_callback(Box::new(move |r| {
            assert_eq!(
                *r.unwrap().end_line(),
                EndReplyLine {
                    code: 250,
                    line: Bytes::from_static(b"OK")
                }
            );
            c2.set(true);
        }));
        dispatcher.feed(&Bytes::from_static(b"250 O")).unwrap();
        assert!(!called.get());
        dispatcher.feed(&Bytes::from_static(b"K\r\n")).unwrap();
        assert!(called.get());
    }

    #[test]
    fn empty_feed() {
        let dispatcher = ReplyDispatcher::new();
        // Test empty feed is tolerated.
        dispatcher.feed(&Bytes::new()).unwrap();

        dispatcher.push_callback(Box::new(|_| {
            unreachable!("This callback should never be called!");
        }));
        dispatcher.feed(&Bytes::from_static(b"250 OK")).unwrap();
        dispatcher.feed(&Bytes::new()).unwrap();
    }

    #[test]
    fn async_notification() {
        let dispatcher = ReplyDispatcher::new();
        let notification_called = Rc::new(Cell::new(false));
        let notification_called2 = notification_called.clone();
        dispatcher.set_async_handler(Box::new(move |r| {
            assert_eq!(
                *r.end_line(),
                EndReplyLine {
                    code: 650,
                    line: Bytes::from_static(b"Notification")
                }
            );
            notification_called2.set(true);
        }));
        let reply_called = Rc::new(Cell::new(false));
        let reply_called2 = reply_called.clone();
        dispatcher.push_callback(Box::new(move |_| {
            reply_called2.set(true);
        }));
        dispatcher
            .feed(&Bytes::from_static(b"650 Notification\r\n"))
            .unwrap();
        assert!(notification_called.get());
        assert!(!reply_called.get());
    }

    #[test]
    fn async_without_handler() {
        let dispatcher = ReplyDispatcher::new();
        dispatcher
            .feed(&Bytes::from_static(b"650 Notification\r\n"))
            .unwrap();
    }

    #[test]
    fn async_replace_handler() {
        let dispatcher = ReplyDispatcher::new();
        dispatcher.set_async_handler(Box::new(move |_| {
            unreachable!("This handler should never be called, as it has been replaced.");
        }));
        let called = Rc::new(Cell::new(false));
        let c = called.clone();
        dispatcher.set_async_handler(Box::new(move |r| {
            assert_eq!(
                *r.end_line(),
                EndReplyLine {
                    code: 650,
                    line: Bytes::from_static(b"Notification")
                }
            );
            c.set(true);
        }));
        assert!(!called.get());
        dispatcher
            .feed(&Bytes::from_static(b"650 Notification\r\n"))
            .unwrap();
        assert!(called.get());
    }

    #[test]
    fn sync_async_single_feed() {
        let dispatcher = ReplyDispatcher::new();
        let sync_called = Rc::new(Cell::new(false));
        let sc = sync_called.clone();
        dispatcher.push_callback(Box::new(move |r| {
            check_250_ok(&r.unwrap(), false);
            sc.set(true);
        }));
        let async_called = Rc::new(Cell::new(false));
        let ac = async_called.clone();
        dispatcher.set_async_handler(Box::new(move |r| {
            assert_eq!(
                *r.end_line(),
                EndReplyLine {
                    code: 650,
                    line: Bytes::from_static(b"Notification")
                }
            );
            ac.set(true);
        }));
        dispatcher
            .feed(&Bytes::from_static(b"250 OK\r\n650 Notification\r\n"))
            .unwrap();
        assert!(sync_called.get());
        assert!(async_called.get());
    }

    #[test]
    fn feed_before_registering() {
        {
            let dispatcher = ReplyDispatcher::new();
            dispatcher.feed(&Bytes::from_static(b"250 OK\r\n")).unwrap();
            let called = Rc::new(Cell::new(false));
            let c = called.clone();
            dispatcher.push_callback(Box::new(move |r| {
                check_250_ok(&r.unwrap(), false);
                c.set(true);
            }));
            assert!(called.get());
        }

        {
            let dispatcher = ReplyDispatcher::new();
            let first = Rc::new(Cell::new(false));
            let f = first.clone();
            dispatcher.push_callback(Box::new(move |r| {
                let reply = r.unwrap();
                assert_eq!(reply.end_line().code, 250);
                assert_eq!(reply.end_line().line, Bytes::from_static(b"Test 1"));
                f.set(true);
            }));
            dispatcher
                .feed(&Bytes::from_static(b"250 Test 1\r\n250 Test 2\r\n"))
                .unwrap();
            assert!(first.get());
            let second = Rc::new(Cell::new(false));
            let s = second.clone();
            dispatcher.push_callback(Box::new(move |r| {
                let reply = r.unwrap();
                assert_eq!(reply.end_line().code, 250);
                assert_eq!(reply.end_line().line, Bytes::from_static(b"Test 2"));
                s.set(true);
            }));
            assert!(second.get());
        }
    }

    #[test]
    fn reentrant() {
        let dispatcher = Rc::new(ReplyDispatcher::new());
        let p2 = dispatcher.clone();
        let calls = Rc::new(Cell::new(0));
        let c1 = calls.clone();
        dispatcher.push_callback(Box::new(move |r| {
            assert_eq!(
                *r.unwrap().end_line(),
                EndReplyLine {
                    code: 250,
                    line: Bytes::from_static(b"OK 1")
                }
            );
            c1.set(c1.get() + 1);

            let c2 = c1.clone();
            p2.push_callback(Box::new(move |r| {
                assert_eq!(
                    *r.unwrap().end_line(),
                    EndReplyLine {
                        code: 250,
                        line: Bytes::from_static(b"OK 2")
                    }
                );
                c2.set(c2.get() + 1);
            }));
            p2.feed(&Bytes::from_static(b"250 OK 2\r\n")).unwrap();
        }));
        dispatcher
            .feed(&Bytes::from_static(b"250 OK 1\r\n"))
            .unwrap();
        assert_eq!(calls.get(), 2);

        let notifications = Rc::new(Cell::new(0));
        let n = notifications.clone();
        let d = dispatcher.clone();
        dispatcher.set_async_handler(Box::new(move |_| {
            n.set(n.get() + 1);
            if n.get() < 2 {
                d.feed(&Bytes::from_static(b"650 Notification 2\r\n"))
                    .unwrap();
            }
        }));
        dispatcher
            .feed(&Bytes::from_static(b"650 Notification 1\r\n"))
            .unwrap();
        assert_eq!(notifications.get(), 2);
    }

    #[test]
    fn fail_all() {
        let dispatcher = ReplyDispatcher::new();
        let called1 = Rc::new(Cell::new(false));
        let called2 = Rc::new(Cell::new(false));
        let c1 = called1.clone();
        let c2 = called2.clone();
        dispatcher.push_callback(Box::new(move |r| {
            assert_eq!(r.unwrap_err(), ReplyError::ConnectionClosed);
            c1.set(true);
        }));
        dispatcher.push_callback(Box::new(move |r| {
            assert_eq!(r.unwrap_err(), ReplyError::ConnectionClosed);
            c2.set(true);
        }));
        dispatcher.fail_all(ReplyError::ConnectionClosed);
        assert!(called1.get());
        assert!(called2.get());
    }

    #[test]
    fn fail_all_clears_dispatch_queue() {
        let dispatcher = ReplyDispatcher::new();
        dispatcher.push_callback(Box::new(move |r| {
            assert_eq!(r.unwrap_err(), ReplyError::BadSeparator(b'|'));
        }));

        assert_eq!(
            dispatcher
                .feed(&Bytes::from_static(b"250 OK\r\n250|bad\r\n"))
                .unwrap_err(),
            ReplyError::BadSeparator(b'|')
        );

        dispatcher.push_callback(Box::new(move |r| {
            assert_eq!(
                *r.unwrap().end_line(),
                EndReplyLine {
                    code: 250,
                    line: Bytes::from_static(b"NEW")
                }
            );
        }));
        dispatcher
            .feed(&Bytes::from_static(b"250 NEW\r\n"))
            .unwrap();
    }

    #[test]
    fn error_propagation() {
        let dispatcher = ReplyDispatcher::new();
        dispatcher.set_async_handler(Box::new(|_| {
            unreachable!("Errors should not be propagated to the async handler.");
        }));
        let first = Rc::new(Cell::new(false));
        let f = first.clone();
        let second = Rc::new(Cell::new(false));
        let s = second.clone();
        dispatcher.push_callback(Box::new(move |r| {
            assert_eq!(r.unwrap_err(), ReplyError::BadSeparator(b'|'));
            assert!(!f.get());
            f.set(true);
        }));
        dispatcher.push_callback(Box::new(move |r| {
            assert_eq!(r.unwrap_err(), ReplyError::BadSeparator(b'|'));
            assert!(!s.get());
            s.set(true);
        }));
        assert_eq!(
            dispatcher
                .feed(&Bytes::from_static(b"250|OK\r\n"))
                .unwrap_err(),
            ReplyError::BadSeparator(b'|')
        );
        assert!(first.get());
        assert!(second.get());
    }
}

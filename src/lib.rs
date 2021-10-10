use std::collections::VecDeque;
use std::sync::{Arc, Condvar};
use std::sync::Mutex;

struct Inner<T> {
    shared: Mutex<VecDeque<T>>,
    cvar: Condvar,
}

impl<T> Inner<T> {
    pub fn new() -> Self {
        let shared = Mutex::new(VecDeque::new());
        let cvar = Condvar::new();
        Self {
            shared,
            cvar,
        }
    }
}

pub struct Sender<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Sender<T> {
    fn new(inner: Arc<Inner<T>>) -> Self {
        Self {
            inner,
        }
    }

    pub fn send(&self, t: T) -> Result<(), ()> {
        self.inner.shared.lock().unwrap().push_back(t);
        self.inner.cvar.notify_one();
        Ok(())
    }
}

pub struct Receiver<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Receiver<T> {
    fn new(inner: Arc<Inner<T>>) -> Self {
        Self {
            inner,
        }
    }

    pub fn recv(&self) -> Result<T, ()> {
        let mut que = self.inner.shared.lock().unwrap();
        while que.is_empty() {
            que = self.inner.cvar.wait(que).unwrap();
        };
        let elem = que.pop_front().unwrap();
        Ok(elem)
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner::new());
    (Sender::new(Arc::clone(&inner)), Receiver::new(Arc::clone(&inner)))
}

#[cfg(test)]
mod tests {
    struct DummyPayload {
    }

    impl DummyPayload {
        fn new() -> Self {
            Self {}
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct DummyPayloadWithValue {
        internal: u32,
    }

    impl DummyPayloadWithValue {
        fn new(internal: u32) -> Self {
            Self {
                internal,
            }
        }
    }

    use super::*;
    #[test]
    fn test_sender_basic_api() {
        let payload1 = DummyPayload::new();
        let payload2 = DummyPayload::new();
        let (sender, _) = channel();
        sender.send(payload1).unwrap();
        sender.send(payload2).unwrap();
    }

    #[test]
    fn test_receiver_basic_api() {
        let (sender, receiver): (_, Receiver<DummyPayload>) = channel();
        sender.send(DummyPayload::new()).unwrap();
        let _: DummyPayload = receiver.recv().unwrap();
    }

    #[test]
    fn test_recv_returns_sent_value() {
        let payload = DummyPayloadWithValue::new(4123);
        let (sender, receiver) = channel();

        sender.send(payload).unwrap();
        let received = receiver.recv().unwrap();

        assert_eq!(received, payload);
    }

    #[test]
    fn test_sender_can_be_sent_to_threads() {
        let (sender, _) = channel();

        std::thread::spawn(move || {
            sender.send(DummyPayload::new()).unwrap();
        });
    }

    #[test]
    fn test_receiver_can_be_sent_to_threads() {
        let (_, receiver): (_, Receiver<DummyPayload>) = channel();

        std::thread::spawn(move || {
            let _ = receiver.recv().unwrap();
        });
    }

    #[test]
    fn test_no_receive_without_send() {
        let (sender, receiver): (_, Receiver<DummyPayload>) = channel();
        let finished_flag = Arc::new(Mutex::new(false));
        let finished_flag2 = Arc::clone(&finished_flag);
        let pair = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        let pair2 = Arc::clone(&pair);

        std::thread::spawn(move || {
            let (lock, cvar) = &*pair2;
            let mut started = lock.lock().unwrap();
            *started = true;
            drop(started);
            cvar.notify_one();

            let res = receiver.recv().unwrap();
            let mut finished = finished_flag2.lock().unwrap();
            *finished = true;
            res
        });

        let (lock, cvar) = &*pair;
        let mut started = lock.lock().unwrap();

        while !*started {
            started = cvar.wait(started).unwrap();
        }

        std::thread::sleep(std::time::Duration::from_millis(2000));
        if *finished_flag.lock().unwrap() {
            panic!("Spawned thread did not block on recv");
        }

        sender.send(DummyPayload::new()).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(2000));
        if !*finished_flag.lock().unwrap() {
            panic!("Spawned thread did not finish");
        }
    }

    #[test]
    fn test_recv_returns_sent_value_threaded() {
        let payload = DummyPayloadWithValue::new(4123);
        let (sender, receiver) = channel();

        std::thread::spawn(move || {
            sender.send(payload).unwrap();
        });

        let handle = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            let received = receiver.recv().unwrap();
            assert_eq!(received, payload);
        });

        handle.join().unwrap();
    }
}

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::{Arc, Condvar};

struct Inner<T> {
    shared: Mutex<VecDeque<T>>,
    cvar: Condvar,
}

impl<T> Inner<T> {
    pub fn new() -> Self {
        let shared = Mutex::new(VecDeque::new());
        let cvar = Condvar::new();
        Self { shared, cvar }
    }
}

pub struct Sender<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Sender<T> {
    fn new(inner: Arc<Inner<T>>) -> Self {
        Self { inner }
    }

    pub fn send(&self, t: T) -> Result<(), ()> {
        self.inner.shared.lock().unwrap().push_back(t);
        self.inner.cvar.notify_one();
        Ok(())
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        let inner = Arc::clone(&self.inner);
        Sender::new(inner)
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        // If strong_count is 2, this means there are two strong references
        // to the inner struct. One is us. If the other is another sender,
        // then it doesn't matter if we notify the cvar, since no one is
        // waiting on it.
        // If the other is the receiver, then it is safe to notify them,
        // since there will be no other senders after we are droped.
        if Arc::strong_count(&self.inner) == 2 {
            self.inner.cvar.notify_one();
        }
    }
}

pub struct Receiver<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Receiver<T> {
    fn new(inner: Arc<Inner<T>>) -> Self {
        Self { inner }
    }

    pub fn recv(&self) -> Result<T, &'static str> {
        let mut que = self.inner.shared.lock().map_err(|_| "lock error")?;
        while que.is_empty() {
            // If strong_count is 1, it means that there are no other
            // senders, no more values will be received from this channel
            if Arc::strong_count(&self.inner) == 1 {
                return Err("no more values");
            }
            que = self.inner.cvar.wait(que).map_err(|_| "wait error")?;
        }
        let elem = que.pop_front().unwrap();
        Ok(elem)
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner::new());
    (
        Sender::new(Arc::clone(&inner)),
        Receiver::new(Arc::clone(&inner)),
    )
}

#[cfg(test)]
mod tests {
    struct DummyPayload {}

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
            Self { internal }
        }
    }

    use super::*;
    use std::sync::Condvar;
    use std::thread::{sleep, spawn};
    use std::time::Duration;

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

        spawn(move || {
            sender.send(DummyPayload::new()).unwrap();
        });
    }

    #[test]
    fn test_receiver_can_be_sent_to_threads() {
        let (_, receiver): (_, Receiver<DummyPayload>) = channel();

        spawn(move || {
            let _ = receiver.recv().unwrap();
        });
    }

    #[test]
    fn test_no_receive_without_send() {
        let (sender, receiver): (_, Receiver<DummyPayload>) = channel();
        let finished_flag = Arc::new(Mutex::new(false));
        let finished_flag2 = Arc::clone(&finished_flag);
        let pair = Arc::new((Mutex::new(false), Condvar::new()));
        let pair2 = Arc::clone(&pair);

        spawn(move || {
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

        sleep(Duration::from_millis(2000));
        if *finished_flag.lock().unwrap() {
            panic!("Spawned thread did not block on recv");
        }

        sender.send(DummyPayload::new()).unwrap();

        sleep(Duration::from_millis(2000));
        if !*finished_flag.lock().unwrap() {
            panic!("Spawned thread did not finish");
        }
    }

    #[test]
    fn test_recv_returns_sent_value_threaded() {
        let payload = DummyPayloadWithValue::new(4123);
        let (sender, receiver) = channel();

        spawn(move || {
            sender.send(payload).unwrap();
        });

        let handle = spawn(move || {
            sleep(Duration::from_millis(1000));
            let received = receiver.recv().unwrap();
            assert_eq!(received, payload);
        });

        handle.join().unwrap();
    }

    #[test]
    fn test_cloning_works() {
        let target_payload = DummyPayloadWithValue::new(32);
        let (sender, receiver) = channel();
        let sender2 = sender.clone();

        sender2.send(DummyPayloadWithValue::new(32)).unwrap();
        assert_eq!(receiver.recv().unwrap(), target_payload);
    }

    #[test]
    fn test_no_hang_on_dropped_sender() {
        let (sender, receiver): (Sender<DummyPayload>, Receiver<DummyPayload>) = channel();
        let s2 = sender.clone();
        drop(sender);

        s2.send(DummyPayload::new()).unwrap();
        assert!(receiver.recv().is_ok());

        drop(s2);
        assert!(receiver.recv().is_err());
    }

    #[test]
    fn test_drop_senders_wakes_receiver() {
        let (sender, receiver): (Sender<DummyPayload>, _) = channel();
        let sender2 = sender.clone();
        let finished = Arc::new(Mutex::new(false));
        let finished2 = Arc::clone(&finished);

        spawn(move || match receiver.recv() {
            Ok(_) => panic!("received value when it shouldn't"),
            Err(msg) => {
                assert_eq!(msg, "no more values");
                *finished2.lock().unwrap() = true;
            }
        });

        sleep(Duration::from_millis(1000));
        assert!(!*finished.lock().unwrap());

        drop(sender);
        sleep(Duration::from_millis(1000));
        assert!(!*finished.lock().unwrap());

        drop(sender2);
        sleep(Duration::from_millis(1000));
        assert!(*finished.lock().unwrap());
    }
}

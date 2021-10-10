use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;

pub struct Sender<T> {
    shared: Arc<Mutex<VecDeque<T>>>,
}

impl<T> Sender<T> {
    pub fn new(shared: Arc<Mutex<VecDeque<T>>>) -> Self {
        Self {
            shared,
        }
    }

    pub fn send(&self, t: T) -> Result<(), ()> {
        self.shared.lock().unwrap().push_back(t);
        Ok(())
    }
}

pub struct Receiver<T> {
    shared: Arc<Mutex<VecDeque<T>>>,
}

impl<T> Receiver<T> {
    pub fn new(shared: Arc<Mutex<VecDeque<T>>>) -> Self {
        Self {
            shared,
        }
    }

    pub fn recv(&self) -> Result<T, ()> {
        let elem = self.shared.lock().unwrap().pop_front().unwrap();
        Ok(elem)
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let shared = Arc::new(Mutex::new(VecDeque::new()));
    (Sender::new(shared.clone()), Receiver::new(shared))
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
        let (_, receiver): (_, Receiver<DummyPayload>) = channel();
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
            *finished_flag2.lock().unwrap() = true;
            res
        });

        let (lock, cvar) = &*pair;
        let mut started = lock.lock().unwrap();

        while !*started {
            started = cvar.wait(started).unwrap();
        }

        if *finished_flag.lock().unwrap() {
            panic!("Spawned thread did not block on recv");
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

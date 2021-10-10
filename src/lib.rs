use std::sync::Arc;
use std::sync::Mutex;

pub struct Sender<T> {
    shared: Arc<Mutex<T>>,
}

impl<T> Sender<T> {
    pub fn new(shared: Arc<Mutex<T>>) -> Self {
        Self {
            shared,
        }
    }

    pub fn send(&self, t: T) -> Result<(), ()> {
        *self.shared.lock().unwrap() = t;
        Ok(())
    }
}

pub struct Receiver<T> {
    shared: Arc<Mutex<T>>,
}

impl<T> Receiver<T> {
    pub fn new(shared: Arc<Mutex<T>>) -> Self {
        Self {
            shared,
        }
    }

    pub fn recv(&self) -> Result<T, ()> {
        Ok(std::mem::replace(&mut self.shared.lock().unwrap(), unsafe { std::mem::zeroed() }))
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let shared = Arc::new(Mutex::new(unsafe { std::mem::zeroed() }));
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
        let (_, receiver): (_, Receiver<DummyPayload>) = channel();
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
}

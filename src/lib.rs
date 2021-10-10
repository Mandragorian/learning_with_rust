use std::rc::Rc;
use std::cell::RefCell;

pub struct Sender<T> {
    shared: Rc<RefCell<T>>,
}

impl<T> Sender<T> {
    pub fn new(shared: Rc<RefCell<T>>) -> Self {
        Self {
            shared,
        }
    }

    pub fn send(&self, t: T) -> Result<(), ()> {
        *self.shared.borrow_mut() = t;
        Ok(())
    }
}

pub struct Receiver<T> {
    shared: Rc<RefCell<T>>,
}

impl<T> Receiver<T> {
    pub fn new(shared: Rc<RefCell<T>>) -> Self {
        Self {
            shared,
        }
    }

    pub fn recv(&self) -> Result<T, ()> {
        Ok(self.shared.replace(unsafe { std::mem::zeroed() }))
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let shared = Rc::new(RefCell::new(unsafe { std::mem::zeroed() }));
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
}

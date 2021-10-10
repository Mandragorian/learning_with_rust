use std::marker::PhantomData;

pub struct Sender<T> {
    _payload: PhantomData<T>,
}

impl<T> Sender<T> {
    pub fn new() -> Self {
        Self {
            _payload: PhantomData,
        }
    }

    pub fn send(&self, _t: T) -> Result<(), ()> {
        Ok(())
    }
}

pub struct Receiver<T> {
    _payload: PhantomData<T>,
}

impl<T> Receiver<T> {
    pub fn new() -> Self {
        Self {
            _payload: PhantomData,
        }
    }

    pub fn recv(&self) -> Result<T, ()> {
        Ok(unsafe { std::mem::zeroed() })
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    (Sender::new(), Receiver::new())
}

#[cfg(test)]
mod tests {
    struct DummyPayload {}

    impl DummyPayload {
        fn new() -> Self {
            Self {}
        }
    }

    use super::*;
    #[test]
    fn sender_basic_api() {
        let payload1 = DummyPayload::new();
        let payload2 = DummyPayload::new();
        let (sender, _) = channel();
        sender.send(payload1).unwrap();
        sender.send(payload2).unwrap();
    }

    #[test]
    fn receiver_basic_api() {
        let (_, receiver): (_, Receiver<DummyPayload>) = channel();
        let res: DummyPayload = receiver.recv().unwrap();
    }
}

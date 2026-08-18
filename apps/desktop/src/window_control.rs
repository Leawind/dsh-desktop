use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type WindowControlConnection = Arc<Mutex<Box<dyn tiny_http::ReadWrite + Send>>>;

#[derive(Clone, Default)]
pub struct WindowControlRegistry(Arc<Mutex<HashMap<String, WindowControlConnection>>>);

impl WindowControlRegistry {
    pub fn connect(&self, label: String, stream: Box<dyn tiny_http::ReadWrite + Send>) {
        self.0
            .lock()
            .expect("window control registry poisoned")
            .insert(label, Arc::new(Mutex::new(stream)));
    }

    pub fn remove(&self, label: &str) {
        self.0
            .lock()
            .expect("window control registry poisoned")
            .remove(label);
    }

    pub fn send_to_all(
        &self,
        send: impl Fn(&mut (dyn tiny_http::ReadWrite + Send)) -> std::io::Result<()>,
    ) {
        let connections = self
            .0
            .lock()
            .expect("window control registry poisoned")
            .iter()
            .map(|(label, stream)| (label.clone(), Arc::clone(stream)))
            .collect::<Vec<_>>();
        let disconnected = connections
            .into_iter()
            .filter_map(|(label, stream)| {
                send(&mut **stream.lock().expect("window control stream poisoned"))
                    .is_err()
                    .then_some(label)
            })
            .collect::<Vec<_>>();
        let mut registry = self.0.lock().expect("window control registry poisoned");
        for label in disconnected {
            registry.remove(&label);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn sends_to_each_connected_window() {
        let controls = WindowControlRegistry::default();
        let main = Arc::new(Mutex::new(Vec::new()));
        let second = Arc::new(Mutex::new(Vec::new()));
        controls.connect("main".to_owned(), Box::new(SharedWriter(Arc::clone(&main))));
        controls.connect(
            "second".to_owned(),
            Box::new(SharedWriter(Arc::clone(&second))),
        );

        controls.send_to_all(|stream| stream.write_all(b"close"));

        assert_eq!(*main.lock().unwrap(), b"close");
        assert_eq!(*second.lock().unwrap(), b"close");
    }

    #[test]
    fn replacing_a_connection_uses_the_new_stream() {
        let controls = WindowControlRegistry::default();
        let old = Arc::new(Mutex::new(Vec::new()));
        let current = Arc::new(Mutex::new(Vec::new()));
        controls.connect("main".to_owned(), Box::new(SharedWriter(Arc::clone(&old))));
        controls.connect(
            "main".to_owned(),
            Box::new(SharedWriter(Arc::clone(&current))),
        );

        controls.send_to_all(|stream| stream.write_all(b"close"));

        assert!(old.lock().unwrap().is_empty());
        assert_eq!(*current.lock().unwrap(), b"close");
    }

    #[test]
    fn removing_a_window_closes_its_channel() {
        let controls = WindowControlRegistry::default();
        let stream = Arc::new(Mutex::new(Vec::new()));
        controls.connect(
            "main".to_owned(),
            Box::new(SharedWriter(Arc::clone(&stream))),
        );

        controls.remove("main");
        controls.send_to_all(|stream| stream.write_all(b"close"));

        assert!(stream.lock().unwrap().is_empty());
    }

    struct SharedWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Read for SharedWriter {
        fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
            Ok(0)
        }
    }
}

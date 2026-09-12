//! A process-owned loopback endpoint that rejects every proxy connection.
//!
//! WebView2 can issue background HTTP requests independently of the page CSP.
//! Keep its proxy endpoint bound for the whole application lifetime so another
//! local service cannot take the port and forward those requests. No request
//! data is read, logged, resolved or forwarded.
use std::io;
use std::net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct OfflineProxy {
    address: SocketAddr,
    stopping: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl OfflineProxy {
    pub fn start() -> io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let address = listener.local_addr()?;
        let stopping = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&stopping);
        let worker = thread::Builder::new()
            .name("offline-webview-proxy".into())
            .spawn(move || {
                while !stop.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let _ = stream.shutdown(Shutdown::Both);
                        }
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                        Err(_) => {
                            // Keep ownership of the port even if accepting fails.
                            thread::sleep(Duration::from_millis(100));
                        }
                    }
                }
            })?;
        Ok(Self {
            address,
            stopping,
            worker: Some(worker),
        })
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.address)
    }
}

impl Drop for OfflineProxy {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        // Wake accept without reading or writing any application data.
        let _ = TcpStream::connect_timeout(&self.address, Duration::from_secs(1));
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn closes_http_and_connect_requests_without_forwarding() {
        let proxy = OfflineProxy::start().unwrap();
        assert!(proxy.address.ip().is_loopback());
        assert!(proxy.url().starts_with("http://127.0.0.1:"));
        for request in [
            "GET http://example.invalid/ HTTP/1.1\r\n\r\n",
            "CONNECT example.invalid:443 HTTP/1.1\r\n\r\n",
        ] {
            let mut stream = TcpStream::connect(proxy.address).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let _ = stream.write_all(request.as_bytes());
            let mut response = [0; 64];
            match stream.read(&mut response) {
                Ok(0) => {}
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionAborted
                    ) => {}
                result => panic!("The proxy must close the connection: {result:?}"),
            }
        }
    }

    #[test]
    fn owns_the_endpoint_until_shutdown_and_releases_it_afterwards() {
        let proxy = OfflineProxy::start().unwrap();
        let address = proxy.address;
        assert!(TcpListener::bind(address).is_err());
        drop(proxy);
        assert!(TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_err());
    }
}

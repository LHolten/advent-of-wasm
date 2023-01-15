use std::thread;

use rusqlite::Connection;
use tokio::sync::{mpsc, oneshot};

type CallFn = Box<dyn FnOnce(&mut Connection) + Send>;

#[derive(Debug, Clone)]
pub struct SharedConnection {
    sender: mpsc::Sender<CallFn>,
}

impl SharedConnection {
    pub fn new(mut conn: Connection) -> Self {
        let (sender, mut receiver) = mpsc::channel::<CallFn>(1);
        thread::spawn(move || {
            while let Some(func) = receiver.blocking_recv() {
                func(&mut conn)
            }
        });
        Self { sender }
    }

    pub async fn call<F, R>(&self, func: F) -> R
    where
        F: FnOnce(&mut Connection) -> R + Send + 'static,
        R: Send + 'static,
    {
        let (sender, receiver) = oneshot::channel::<R>();
        let wrapper: CallFn = Box::new(move |conn| {
            let res = func(conn);
            // since res does not implement Debug, we can not get more info
            assert!(sender.send(res).is_ok());
        });
        // the closure does not implement Debug either.
        assert!(self.sender.send(wrapper).await.is_ok());
        receiver.await.unwrap()
    }
}

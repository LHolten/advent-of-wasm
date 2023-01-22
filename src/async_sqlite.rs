use std::{
    ops::Deref,
    sync::{Arc, Condvar, Mutex, MutexGuard},
    thread,
};

use rusqlite::Connection;
use tokio::sync::{mpsc, oneshot};

type CallFn = Box<dyn FnOnce(&mut Connection) + Send>;

#[derive(Debug, Clone)]
pub struct SharedConnection {
    sender: mpsc::Sender<CallFn>,
    inner: Arc<Inner>,
}

#[derive(Debug)]
pub struct Inner {
    conn: Mutex<Connection>,
    cvar: Condvar,
    updates: Mutex<bool>,
}

impl SharedConnection {
    pub fn new(conn: Connection) -> Self {
        let (sender, mut receiver) = mpsc::channel::<CallFn>(1);
        let inner = Arc::new(Inner {
            conn: Mutex::new(conn),
            cvar: Condvar::new(),
            updates: Mutex::new(true),
        });
        let res = Self {
            sender,
            inner: inner.clone(),
        };
        thread::spawn(move || {
            while let Some(func) = receiver.blocking_recv() {
                func(&mut inner.lock());
                *inner.updates.lock().unwrap() = true;
                inner.cvar.notify_all();
            }
        });
        res
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

impl Deref for SharedConnection {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Inner {
    // // this will not change updates
    // pub fn call_blocking<F, R>(&self, func: F) -> R
    // where
    //     F: FnOnce(&mut Connection) -> R + Send,
    //     R: Send,
    // {
    //     let mut conn = self.conn.lock().unwrap();
    //     func(&mut conn)
    // }

    pub fn lock(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }

    pub fn wait(&self) {
        let updates = self.updates.lock().unwrap();
        *self.cvar.wait_while(updates, |&mut x| !x).unwrap() = false;
    }
}

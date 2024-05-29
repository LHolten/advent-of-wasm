use std::{
    ops::Deref,
    sync::{Arc, Condvar, LazyLock, Mutex, MutexGuard},
    thread,
};

pub static DB: LazyLock<SharedConnection> = LazyLock::new(|| {
    let (client, schema) = initialize_db();
    SharedConnection::new(client, schema)
});

use rust_query::client::Client;
use tokio::sync::{mpsc, oneshot};

use crate::migration::{initialize_db, Schema};

type CallFn = Box<dyn FnOnce(&mut Client) + Send>;

#[derive(Clone)]
pub struct SharedConnection {
    sender: mpsc::Sender<CallFn>,
    inner: Arc<Inner>,
}

pub struct Inner {
    conn: Mutex<Client>,
    cvar: Condvar,
    updates: Mutex<bool>,
    schema: Schema,
}

impl SharedConnection {
    pub fn new(conn: Client, schema: Schema) -> Self {
        let (sender, mut receiver) = mpsc::channel::<CallFn>(1);
        let inner = Arc::new(Inner {
            conn: Mutex::new(conn),
            cvar: Condvar::new(),
            updates: Mutex::new(true),
            schema,
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
        F: FnOnce(&mut Client) -> R + Send + 'static,
        R: Send + 'static,
    {
        let (sender, receiver) = oneshot::channel::<R>();
        let wrapper: CallFn = Box::new(move |conn| {
            let res = func(conn);
            // since res does not implement Debug, we can not get more info
            let _ = sender.send(res);
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

impl Deref for Inner {
    type Target = Schema;

    fn deref(&self) -> &Self::Target {
        &self.schema
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

    pub fn lock(&self) -> MutexGuard<'_, Client> {
        self.conn.lock().unwrap()
    }

    pub fn wait(&self) {
        let updates = self.updates.lock().unwrap();
        *self.cvar.wait_while(updates, |&mut x| !x).unwrap() = false;
    }
}

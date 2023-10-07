
use actix_web::dev::{ServerHandle};
use error_mapper::{map_to_new_error, TheResult};
use lazy_static::lazy_static;
use tokio::sync::broadcast::Receiver;
use tokio::sync::RwLock;
use crate::StopMethod;

lazy_static!{
    static ref SHUTTING_DOWN: Shutdown = Shutdown::new();
}

pub struct Shutdown {
    inner: RwLock<ShutdownInner>
}

struct ShutdownInner {
    shutting_down: bool
}

impl Shutdown {
    fn new() -> Self {
        Self {
            inner: RwLock::new(ShutdownInner {
                shutting_down: false
            })
        }
    }

    pub fn instance() -> &'static Self {
        &SHUTTING_DOWN
    }

    pub async fn shutdown_started(&self) {
        self.inner.write().await.shutting_down = true
    }

    pub async fn is_shutting_down(&self) -> bool {
        self.inner.read().await.shutting_down
    }

    pub async fn set_shutdown_state_to_false(&self) {
        self.inner.write().await.shutting_down = false
    }
}

pub async fn stop_server(server: ServerHandle, mut receiver: Receiver<StopMethod>) -> TheResult<()>{

    match receiver.recv().await {
        Ok(StopMethod::Graceful) => {
            Shutdown::instance().shutdown_started().await;
            server.stop(true).await;
            Ok(())
        },
        Ok(StopMethod::Immediate) => {
            Shutdown::instance().shutdown_started().await;
            server.stop(false).await;
            Ok(())
        },
        Err(e) => {
            Shutdown::instance().shutdown_started().await;
            server.stop(false).await;
            Err(map_to_new_error!(e))
        }
    }
}

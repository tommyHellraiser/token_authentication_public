
use lazy_static::lazy_static;
use tokio::sync::broadcast::Receiver;
use crate::api::StopMethod;

mod queries;

lazy_static!{
    // static ref CRONS_TASK_COUNTER: CronsTaskCounter = CronsTaskCounter::new();
    static ref COUNTER: tokio::sync::Mutex<u8> = tokio::sync::Mutex::new(0);
}

pub mod sessions;

pub async fn run_crons(stopper: Receiver<StopMethod>) {
    tokio::spawn(sessions::close_expired_sessions(stopper.resubscribe()));
}


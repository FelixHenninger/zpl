use super::{Label, ShutdownToken};

use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
};

/// A unique physical printer device.
///
/// We assume to operate it as an owner, when a connection to it can be made.
pub struct PhysicalPrinter {
    label: Label,
}

pub struct Driver {
    message: mpsc::Receiver<Task>,
    end: oneshot::Receiver<ShutdownToken>,
}

pub enum Task {}

impl PhysicalPrinter {
    pub async fn operate(self, driver: Driver) {
        let mut label_being_printed: JoinSet<tokio::task::JoinHandle<()>> =
            JoinSet::new();
        let mut driver = driver;

        loop {
            tokio::select!(
                _  = label_being_printed.join_next(), if !label_being_printed.is_empty() => {
                },
                // Back-Pressure: only accept message while not printing. Could also do a buffer
                // but the channel already is a buffer itself. That only makes sense if we want to
                // do a re-ordering that the channel's sequential semantics does not permit.
                task = driver.message.recv(), if label_being_printed.is_empty() => {
                    todo!()
                }
            )
        }
    }
}

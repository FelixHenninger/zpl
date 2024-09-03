use super::ShutdownToken;
use crate::configuration::LabelPrinter;
use crate::job::PrintJob;

use log::{debug, error, info};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
};

use zpl::{command::HostStatus, device::ZplPrinter};

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// A unique physical printer device.
///
/// We assume to operate it as an owner, when a connection to it can be made. This is a
/// pre-configured builder structure for an operating connection.
#[derive(Clone)]
pub struct PhysicalPrinter {
    label: Arc<LabelPrinter>,
    status: Arc<PrinterStatus>,
}

#[derive(Default)]
struct PrinterStatus {
    is_up: AtomicBool,
}

#[derive(serde::Serialize)]
pub struct StatusInformation {
    is_up: bool,
}

pub struct Driver {
    message: mpsc::Sender<Task>,
    end: Option<oneshot::Sender<ShutdownToken>>,
}

/// Counter part to driver, the physical printer side.
pub struct Connector {
    message: mpsc::Receiver<Task>,
    end: oneshot::Receiver<ShutdownToken>,
    name: String,
}

pub enum Task {
    Job(PrintJob),
}

struct ActiveConnection {
    label: Arc<LabelPrinter>,
    printer: ZplPrinter,
    device_status: HostStatus,
}

impl PhysicalPrinter {
    pub fn new(label: LabelPrinter) -> Self {
        PhysicalPrinter {
            label: Arc::new(label),
            status: Arc::default(),
        }
    }

    pub fn status(&self) -> StatusInformation {
        StatusInformation {
            is_up: self.status.is_up.load(Ordering::Relaxed),
        }
    }

    pub async fn drive(self, con: Connector) {
        let mut label_being_printed: JoinSet<anyhow::Result<ActiveConnection>> =
            JoinSet::new();
        let mut con = con;
        let mut active: Option<ActiveConnection> = None;

        let retry_fail = std::time::Duration::from_millis(1_000);

        loop {
            if label_being_printed.is_empty() && active.is_none() {
                info!(
                    "[{}]: Connecting to printer at {}",
                    con.name, self.label.addr
                );
                let label = self.label.clone();
                let status = self.status.clone();
                let name = con.name.clone();

                label_being_printed.spawn(async move {
                    let mut printer =
                        ZplPrinter::with_address(label.addr).await?;
                    debug!("[{}]: Connection opened", name);
                    let device_status = printer.request_device_status().await?;
                    info!("[{}]: Device status up", name);

                    status
                        .is_up
                        .fetch_or(true, std::sync::atomic::Ordering::Relaxed);

                    let device_status = device_status.clone();

                    Ok(ActiveConnection {
                        printer,
                        device_status,
                        label,
                    })
                });
            }

            tokio::select!(
                success = label_being_printed.join_next(), if !label_being_printed.is_empty() => {
                    match success {
                        Some(Ok(Ok(ready))) => {
                            info!("[{}]: Printer ready for label at {}", con.name, self.label.addr);
                            active = Some(ready);
                        },
                        Some(Ok(Err(err))) => {
                            debug!("[{}]: {:?}", con.name, err);
                            debug!("[{}]: Retry in {:?}", con.name, retry_fail);
                            tokio::time::sleep(retry_fail).await;
                        }
                        Some(Err(err)) => {
                            error!("[{}]: {:?}", con.name,  err);
                        }
                        None => unreachable!(),
                    }
                },
                end = &mut con.end => {
                    match end {
                        Ok(ShutdownToken) => {},
                        Err(recv_error) => {
                            error!("[{}]: {recv_error:?}", con.name);
                        }
                    }

                    break;
                },
                // Back-Pressure: only accept message while not printing. Could also do a buffer
                // but the channel already is a buffer itself. That only makes sense if we want to
                // do a re-ordering that the channel's sequential semantics does not permit.
                job = con.message.recv(), if label_being_printed.is_empty() => {
                    let Some(Task::Job(job)) = job else { break; };
                    let active = active.take().unwrap();
                    label_being_printed.spawn(print_label(active, job));
                }
            )
        }
    }
}

async fn print_label(
    mut con: ActiveConnection,
    job: PrintJob,
) -> anyhow::Result<ActiveConnection> {
    let label = tokio::task::block_in_place(|| {
        job.into_label(&con.label.dimensions, &con.device_status.identification)
    });

    let seq = label.print(1).await?;
    // tokio::fs::write("/tmp/zpl-debug", seq.to_string()).await?;
    let _ = con.printer.send(seq).await?;
    // No change in connection state, free to reuse it.
    Ok(con)
}

impl Driver {
    pub fn new(printer: &LabelPrinter) -> (Self, Connector) {
        // Aggressive bound on queue length.
        const BOUND: usize = 8;

        let (msg_send, msg_recv) = mpsc::channel(BOUND);
        let (end_send, end_recv) = oneshot::channel();

        let driver = Driver {
            message: msg_send,
            end: Some(end_send),
        };

        let con = Connector {
            message: msg_recv,
            end: end_recv,
            name: format!("@{}", printer.addr),
        };

        (driver, con)
    }

    pub async fn send_job(&self, job: PrintJob) -> Result<(), &'static str> {
        match self.message.try_send(Task::Job(job)) {
            Ok(_) => Ok(()),
            Err(_) => Err("failed to queue"),
        }
    }

    pub fn shutdown(&mut self) {
        if let Some(sender) = self.end.take() {
            let _ = sender.send(ShutdownToken);
        }
    }
}

impl Connector {
    pub fn with_name(self, name: String) -> Self {
        Connector { name, ..self }
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        self.shutdown();
    }
}

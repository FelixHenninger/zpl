use super::ShutdownToken;
use crate::{configuration, job};

use log::{debug, error, info};

use serde::Serialize;

use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
};

use zpl::{command::HostStatus, device::ZplPrinter};

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct LabelPrinter {
    config: Arc<configuration::LabelPrinter>,
    label: Arc<configuration::Label>,
}

/// A unique physical printer device.
///
/// We assume to operate it as an owner, when a connection to it can be made. This is a
/// pre-configured builder structure for an operating connection.
#[derive(Clone)]
pub struct PhysicalPrinter {
    target: Arc<LabelPrinter>,
    status: Arc<PrinterStatus>,
}

#[derive(Default)]
struct PrinterStatus {
    is_up: AtomicBool,
}

#[derive(Serialize)]
pub struct StatusInformation {
    display_name: Option<String>,
    printer_label: PrinterInformation,
    is_up: bool,
}

struct PrinterInformation(Arc<LabelPrinter>);

impl Serialize for PrinterInformation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Only the dimensions are public information.
        self.0.label.dimensions.serialize(serializer)
    }
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
    Job(job::PrintJob),
}

struct ActiveConnection {
    target: Arc<LabelPrinter>,
    printer: ZplPrinter,
    device_status: HostStatus,
}

impl LabelPrinter {
    pub fn new(
        cfg: &configuration::Configuration,
        printer: Arc<configuration::LabelPrinter>,
    ) -> Option<Self> {
        let label = cfg.labels.get(&printer.label)?;
        Some(LabelPrinter {
            config: printer,
            label: label.clone(),
        })
    }
}

impl PhysicalPrinter {
    pub fn new(label: LabelPrinter) -> Self {
        PhysicalPrinter {
            target: Arc::new(label),
            status: Arc::default(),
        }
    }

    /// Get the serializable public status information for this printer.
    pub fn status(&self) -> StatusInformation {
        StatusInformation {
            is_up: self.status.is_up.load(Ordering::Relaxed),
            display_name: self.target.config.display_name.clone(),
            printer_label: PrinterInformation(self.target.clone()),
        }
    }

    pub async fn verify_label(
        &self,
        payload: &job::PrintApi,
    ) -> Result<job::PrintJob, String> {
        if let Some(dimensions) = &payload.dimensions {
            if !dimensions.approx_cmp(&self.target.label.dimensions) {
                return Err(
                    "Dimension mismatch, check physical label configuration"
                        .to_string(),
                );
            }
        };

        match tokio::task::block_in_place(|| payload.validate_as_job()) {
            Ok(job) => Ok(job),
            Err(error) => return Err(error.to_string()),
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
                    con.name, self.target.config.addr
                );
                let label = self.target.clone();
                let status = self.status.clone();
                let name = con.name.clone();

                label_being_printed.spawn(async move {
                    let mut printer =
                        ZplPrinter::with_address(label.config.addr).await?;
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
                        target: label,
                    })
                });
            }

            tokio::select!(
                success = label_being_printed.join_next(), if !label_being_printed.is_empty() => {
                    match success {
                        Some(Ok(Ok(ready))) => {
                            info!("[{}]: Printer ready for label at {}", con.name, self.target.config.addr);
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
    job: job::PrintJob,
) -> anyhow::Result<ActiveConnection> {
    let label = tokio::task::block_in_place(|| {
        job.into_label(
            &con.target.label.dimensions,
            &con.device_status.identification,
        )
    });

    let seq = label.print(1).await?;
    // tokio::fs::write("/tmp/zpl-debug", seq.to_string()).await?;
    con.printer.send(seq).await?;

    // No change in connection state, free to reuse it.
    Ok(con)
}

impl Driver {
    pub fn new(target: &LabelPrinter) -> (Self, Connector) {
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
            name: format!("@{}", target.config.addr),
        };

        (driver, con)
    }

    pub async fn send_job(
        &self,
        job: job::PrintJob,
    ) -> Result<(), &'static str> {
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

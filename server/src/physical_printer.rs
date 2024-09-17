use crate::{configuration, job, ShutdownToken};

use log::{debug, error, info, warn};

use serde::Serialize;

use std::{
    io::Write as _,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
};

use zpl::{
    command::{HostIdentification, HostStatus},
    device::ZplPrinter,
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
    // FIXME: this can get out of date. Quickly. If the connection itself is not used then we might
    // not even realize. We should periodically re-validate (possibly by 'heartbeat' messages, or
    // TCP socket stats) and additionally the time of last contact could be remembered to provide
    // an accurate picture of reliability to clients.
    is_up: AtomicBool,
}

#[derive(Serialize)]
pub struct StatusInformation {
    display_name: Option<String>,
    printer_label: PrinterInformation,
    is_up: bool,
}

type ConnectionHandled = anyhow::Result<Option<ActiveConnection>>;

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
    Job { print_job: job::PrintJob },
}

struct ActiveConnection {
    target: Arc<LabelPrinter>,
    printer: ZplPrinter,
    device_status: HostStatus,
}

struct SimulationParameter {
    wait_time: std::time::Duration,
    dpmm: Option<u32>,
    target: Arc<LabelPrinter>,
    persist: Option<PathBuf>,
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
        let mut label_being_printed: JoinSet<ConnectionHandled> =
            JoinSet::new();
        let mut con = con;
        let mut active: Option<ActiveConnection> = None;

        // To avoid barraging the printer / network with connection attempts, we ensure a minimum
        // amount of time is between each one. Note that these refer to the connection attempt
        // itself, meaning if a connection is running for a longtime and then drops the first
        // attempt is made immediately. This then restarts the delay.
        //
        // Note: the first tick is immediate meaning we do not wait with the initial connection.
        let retry_fail = std::time::Duration::from_millis(1_000);
        let mut interval_reconnect = tokio::time::interval(retry_fail);
        interval_reconnect
            .set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            if label_being_printed.is_empty()
                && active.is_none()
                && self.target.config.virtualization.is_connnected()
            {
                interval_reconnect.tick().await;

                info!(
                    "[{}]: Connecting to printer at {}",
                    con.name, self.target.config.addr
                );

                info!(
                    "[{}]: Next reconnection attempt in {:?}",
                    con.name, retry_fail
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

                    Ok(Some(ActiveConnection {
                        printer,
                        device_status,
                        target: label,
                    }))
                });
            }

            let is_connection_busy = !label_being_printed.is_empty();

            tokio::select!(
                success = label_being_printed.join_next(), if is_connection_busy => {
                    match success {
                        Some(Ok(Ok(ready))) => {
                            if ready.is_some() {
                                info!("[{}]: Ready for next label in a few", con.name);
                            }

                            active = ready;
                        },
                        Some(Ok(Err(err))) => {
                            warn!("[{}]: {:?}", con.name, err);
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
                job = con.message.recv(), if !is_connection_busy => {
                    let Some(Task::Job{ print_job }) = job else {
                        // Reached end of job queue.
                        break;
                    };

                    self.create_job(print_job, active.take(), &mut label_being_printed);
                }
            )
        }
    }

    fn create_job(
        &self,
        print_job: job::PrintJob,
        con: Option<ActiveConnection>,
        label_being_printed: &mut JoinSet<ConnectionHandled>,
    ) {
        match &self.target.config.virtualization {
            configuration::LabelVirtualization::DropJobs {
                wait_time,
                persist,
            } => {
                let simulation = SimulationParameter {
                    wait_time: *wait_time,
                    dpmm: None,
                    target: self.target.clone(),
                    persist: persist.clone(),
                };

                label_being_printed
                    .spawn(simulation_label(con, print_job, simulation));
            }
            configuration::LabelVirtualization::ZplOnly {
                dpmm,
                persist,
                wait_time,
            } => {
                let simulation = SimulationParameter {
                    wait_time: *wait_time,
                    dpmm: *dpmm,
                    target: self.target.clone(),
                    persist: persist.clone(),
                };

                label_being_printed
                    .spawn(simulation_label(con, print_job, simulation));
            }
            configuration::LabelVirtualization::Physical => {
                let active = con
                    .expect("Pyshical connection re-spawned or still active");
                label_being_printed.spawn(print_label(active, print_job));
            }
        }
    }
}

async fn print_label(
    mut con: ActiveConnection,
    job: job::PrintJob,
) -> ConnectionHandled {
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
    Ok(Some(con))
}

async fn simulation_label(
    con: Option<ActiveConnection>,
    job: job::PrintJob,
    sim: SimulationParameter,
) -> ConnectionHandled {
    let SimulationParameter {
        dpmm,
        mut persist,
        target,
        wait_time,
    } = sim;

    // Start the time for our operation, do not depend on conversion itself.
    let target_time = tokio::time::sleep(wait_time);

    let identification = if let Some(con) = &con {
        con.device_status.identification.clone()
    } else {
        let mut host = HostIdentification::default();

        if let Some(dpmm) = dpmm {
            host.dpmm = dpmm;
        } else {
            warn!("No dpmm configured, nor discovered from the printer. Using 8 dpmm");
            host.dpmm = 8;
        }

        host
    };

    let label = tokio::task::block_in_place(|| {
        job.into_label(&target.label.dimensions, &identification)
    });

    let commands = label.render().await?;
    // Loop once but also can break..
    while let Some(target) = persist.take() {
        let into = match tempfile::Builder::new()
            .prefix(&format!("label-{}-", {
                std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .map_or(0, |duration| duration.as_secs())
            }))
            .suffix(".zpl")
            .tempfile_in(&target)
        {
            Ok(file) => file,
            Err(error) => {
                warn!("Failed to dump ZPL even through requested: {error}");
                break;
            }
        };

        info!("Persisting ZPL into {}", into.path().display());

        if let Err(error) = write!(&into, "{}", commands) {
            warn!("Failed to dump ZPL even through requested: {error}");
            break;
        }

        let path = into.path().to_owned();
        if let Err(error) = into.persist(&path) {
            warn!("Failed to persist ZPL file: {error}");
            break;
        }

        info!("Persisted ZPL into {}", path.display());
    }

    target_time.await;

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
        print_job: job::PrintJob,
    ) -> Result<(), &'static str> {
        match self.message.try_send(Task::Job { print_job }) {
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

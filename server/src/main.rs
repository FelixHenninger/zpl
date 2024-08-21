mod configuration;
mod job;
mod physical_printer;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};

use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::{sync::RwLock, task::JoinSet};

pub struct ShutdownToken;

#[derive(Clone)]
pub struct Server {
    inner: Arc<RwLock<PrintResources>>,
}

struct PrintResources {
    configuration: PathBuf,
    active_printer: JoinSet<()>,
    printer: HashMap<String, PrintQueue>,
}

struct PrintQueue {
    printer: physical_printer::PhysicalPrinter,
    driver: physical_printer::Driver,
}

async fn reload(State(state): State<Server>) -> &'static str {
    let Ok(configuration) = ({
        let state = state.inner.read().await;
        configuration::Configuration::from_file(&state.configuration).await
    }) else {
        return "Fail";
    };

    let mut state = state.inner.write().await;

    // Drop all standing connections.
    state.printer.clear();
    // And wait for them to finalize, just because. Dropping should be fine but isn't as nice. They
    // should be dropped by themselves? Maybe we should just shove them into the background.
    while let Some(_next) = state.active_printer.join_next().await {}

    for (name, configuration) in configuration.labels {
        let (driver, con) = physical_printer::Driver::new(&configuration);
        let printer = physical_printer::PhysicalPrinter::new(configuration);

        let con = con.with_name(name.clone());
        state.active_printer.spawn(printer.clone().drive(con));

        let queue = PrintQueue { printer, driver };
        state.printer.insert(name.clone(), queue);
    }

    "Success"
}

async fn push_job(
    State(state): State<Server>,
    Path(printer): Path<String>,
    Json(payload): Json<job::PrintJob>,
) -> &'static str {
    let inner = state.inner.read().await;
    let Some(queue) = inner.printer.get(&printer) else {
        return "No such printer";
    };

    match queue.driver.send_job(payload).await {
        Ok(()) => "ok",
        Err(err) => err,
    }
}

async fn status(State(state): State<Server>) -> String {
    let inner = state.inner.read().await;

    let map: HashMap<String, serde_json::Value> = inner
        .printer
        .iter()
        .map(|(name, queue)| {
            let description = serde_json::to_value(queue.printer.status())
                .unwrap_or_default();
            (name.clone(), description)
        })
        .collect();

    serde_json::to_string(&map).unwrap()
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let state = Server::new("server.json".into());

    reload(State(state.clone())).await;

    let app = Router::new()
        .route("/info", get(status))
        .route("/reload", post(reload))
        .route("/print/:printer", post(push_job))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap()
}

impl Server {
    pub fn new(configuration: PathBuf) -> Self {
        Server {
            inner: Arc::new(RwLock::new(PrintResources {
                configuration,
                active_printer: Default::default(),
                printer: Default::default(),
            })),
        }
    }
}

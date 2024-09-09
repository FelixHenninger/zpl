mod app;
mod configuration;
mod data_uri;
mod job;
mod physical_printer;
mod spa;

use crate::app::App;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use clap::Parser;

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

async fn reload(State(state): State<Server>) -> String {
    let configuration = match {
        let state = state.inner.read().await;
        configuration::Configuration::from_file(&state.configuration).await
    } {
        Ok(cfg) => cfg,
        Err(error) => {
            return error.to_string();
        }
    };

    let mut state = state.inner.write().await;

    // Drop all standing connections.
    state.printer.clear();
    // And wait for them to finalize, just because. Dropping should be fine but isn't as nice. They
    // should be dropped by themselves? Maybe we should just shove them into the background.
    while let Some(_next) = state.active_printer.join_next().await {}

    for (name, printer) in &configuration.printers {
        let Some(printer) = physical_printer::LabelPrinter::new(
            &configuration,
            printer.0.clone(),
        ) else {
            continue;
        };

        let (driver, con) = physical_printer::Driver::new(&printer);
        let printer = physical_printer::PhysicalPrinter::new(printer);

        let con = con.with_name(name.clone());
        state.active_printer.spawn(printer.clone().drive(con));

        let queue = PrintQueue { printer, driver };
        state.printer.insert(name.clone(), queue);
    }

    "Success".to_string()
}

async fn push_job(
    State(state): State<Server>,
    Path(printer): Path<String>,
    Json(payload): Json<job::PrintApi>,
) -> String {
    let inner = state.inner.read().await;
    let Some(queue) = inner.printer.get(&printer) else {
        return "No such printer".to_string();
    };

    let job = match queue.printer.verify_label(&payload).await {
        Ok(job) => job,
        Err(err) => return err,
    };

    match queue.driver.send_job(job).await {
        Ok(()) => "ok".to_string(),
        Err(err) => err.to_string(),
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
    env_logger::init();

    let config = App::parse();
    let state = Server::new(config.configuration.into());

    assert_eq!(reload(State(state.clone())).await, "Success");

    let app = Router::new()
        .route("/", get(spa::frontpage))
        .route("/index.html", get(spa::frontpage))
        .route("/static/style.css", get(spa::static_style_css))
        .route("/api/v1/info", get(status))
        .route("/api/v1/reload", post(reload))
        .route("/api/v1/print/:printer", post(push_job))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.listen).await.unwrap();
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

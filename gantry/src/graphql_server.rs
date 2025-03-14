use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;

use juniper_graphql_ws::ConnectionConfig;
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::error::TryRecvError;

use axum::routing::{MethodFilter, get, on};
use axum::{Extension, Router};
use futures::{Stream, stream::BoxStream};
use juniper::{
    EmptyMutation, FieldError, GraphQLEnum, GraphQLObject, graphql_object, graphql_subscription,
};

use crate::printer::Instance;

/// define type for schema
type Schema = juniper::RootNode<'static, Query, EmptyMutation, Subscription>;

/// create router for graphql service
pub fn create_router() -> Router {
    let schema = juniper::RootNode::new(Query, EmptyMutation::<()>::new(), Subscription);

    Router::new()
        .route(
            "/graphql",
            on(
                MethodFilter::GET.or(MethodFilter::POST),
                juniper_axum::graphql::<Arc<Schema>>,
            ),
        )
        .route(
            "/subscriptions",
            get(juniper_axum::ws::<Arc<Schema>>(ConnectionConfig::new(()))),
        )
        .route(
            "/graphiql",
            get(juniper_axum::graphiql("/graphql", "/subscriptions")),
        )
        .route(
            "/playground",
            get(juniper_axum::playground("/graphql", "/subscriptions")),
        )
        .layer(Extension(Arc::new(schema)))
}

async fn find_instance(name: &str) -> Option<Arc<Instance>> {
    let insts = crate::INSTANCES.read().await;

    let i = insts.get(name);

    return i.map(|i| i.clone());
}

#[derive(Clone, Copy, Debug)]
pub struct Query;

#[graphql_object]
impl Query {
    /// server
    pub async fn server(&self) -> Server {
        Server
    }

    /// all avaliable printers
    pub async fn printers(&self) -> Vec<Printer> {
        // acquire read lock
        let insts = crate::INSTANCES.read().await;
        // reference all instances
        return insts
            .values()
            .map(|i| Printer {
                instance: i.clone(),
            })
            .collect();
    }

    /// printer with corresponding name
    pub async fn printer(&self, name: String) -> Option<Printer> {
        let instance = find_instance(&name).await?;

        return Some(Printer { instance });
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Server;

#[graphql_object]
impl Server {
    pub async fn info(&self) -> ServerInfo {
        return ServerInfo;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ServerInfo;

#[graphql_object]
impl ServerInfo {
    pub async fn printers(&self) -> Vec<String> {
        todo!()
    }
}

#[derive(Clone)]
pub struct Printer {
    instance: Arc<Instance>,
}

#[graphql_object]
impl Printer {
    /// state of the printer, one of 'startup', 'ready', 'error' or 'shutdown'
    pub async fn state(&self) -> PrinterState {
        match self.instance.state().await {
            crate::printer::State::Startup => PrinterState::Startup,
            crate::printer::State::Ready => PrinterState::Ready,
            crate::printer::State::Shutdown => PrinterState::Shutdown,
            crate::printer::State::Error { .. } => PrinterState::Error_,
        }
    }

    /// emergency stop, stops the printer immediately
    pub async fn emergency_stop(&self) -> bool {
        self.instance.emergency_stop().await;
        return true;
    }
}

#[derive(Debug, Clone, Copy, GraphQLEnum)]
pub enum PrinterState {
    Startup,
    Ready,
    #[graphql(name = "Error")]
    Error_,
    Shutdown,
}

type SubStream<T> = BoxStream<'static, Result<T, FieldError>>;

pub struct BroadcastRecvStream<T> {
    inner: Receiver<T>,
}

impl<T> Unpin for BroadcastRecvStream<T> {}

impl<T> BroadcastRecvStream<T> {
    pub fn new(recv: Receiver<T>) -> Self {
        Self { inner: recv }
    }
}

impl<T: Clone> Stream for BroadcastRecvStream<T> {
    type Item = Result<T, FieldError>;

    fn poll_next(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let stream = self.get_mut();

        match stream.inner.try_recv() {
            Ok(v) => Poll::Ready(Some(Ok(v))),
            Err(TryRecvError::Closed) => Poll::Ready(None),
            Err(TryRecvError::Empty) => Poll::Pending,
            Err(TryRecvError::Lagged(_)) => Poll::Pending,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Subscription;

#[graphql_subscription]
impl Subscription {
    async fn printer_added(&self) -> SubStream<Printer> {
        todo!()
    }

    async fn printer_removed(&self) -> SubStream<Printer> {
        todo!()
    }

    /// printer is ready.
    /// if argument 'printer' is specified, only notify for that printer
    async fn printer_ready(&self, printer: Option<String>) -> SubStream<Printer> {
        // only subscibe to one printer
        if let Some(name) = &printer{
            match find_instance(name).await{
                Some(inst) => todo!(),
                None => return Box::pin(futures::stream::empty())
            }
        }

        todo!()
    }

    async fn printer_error(&self, printer: Option<String>) -> SubStream<Printer> {
        todo!()
    }

    async fn printer_shutdown(&self, printer: Option<String>) -> SubStream<Printer> {
        todo!()
    }

    /// printer is restarting.
    /// if argument 'printer' is specified, only notify for that printer
    async fn printer_restart(&self, printer: Option<String>) -> SubStream<Printer> {
        todo!()
    }

    async fn file_changed(&self, printer: Option<String>) -> SubStream<FileChangeEvent> {
        todo!()
    }

    async fn print_job_start(&self, printer: Option<String>) -> SubStream<PrintJob> {
        todo!()
    }

    async fn print_job_end(&self, printer: Option<String>) -> SubStream<PrintJob> {
        todo!()
    }

    async fn print_job_pause(&self, printer: Option<String>) -> SubStream<PrintJob> {
        todo!()
    }

    async fn printe_job_queue(&self, printer: Option<String>) -> SubStream<PrintJob> {
        todo!()
    }

    /// reports print job progress every interval
    async fn print_job_progress(
        &self,
        printer: Option<String>,
        #[graphql(default = 1000, desc = "interval in ms at which progress is sent")] interval: i32,
    ) -> BoxStream<'static, Result<PrintJob, FieldError>> {
        let interval = interval.min(10);

        let stream = async_stream::stream! {
            loop{
                yield Ok(PrintJob{path: String::new()})
            }
        };

        return Box::pin(stream)
    }
}

#[derive(Debug, Clone, GraphQLEnum)]
pub enum FileChangeEventKind {
    /// file has been modified
    Modified,
    /// file has been created
    Create,
    /// file has been removed
    Removed,
}

/// a file change event
#[derive(Debug, Clone, GraphQLObject)]
pub struct FileChangeEvent {
    /// kind of file change
    pub kind: FileChangeEventKind,
    /// file name
    pub path: String,
}

#[derive(Debug, Clone, GraphQLObject)]
pub struct PrintJob {
    /// gcode filename of the print job
    pub path: String,
}

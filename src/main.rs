use std::{collections::HashMap, convert::Infallible, net::SocketAddr, thread, time::Duration};

use hyper::{
    server::Server,
    service::{make_service_fn, service_fn},
    Body, Client, Request, Response, Uri,
};
use opentelemetry::global;
use opentelemetry_jaeger::{Propagator, Uninstall};
use structopt::StructOpt;
use tracing::subscriber;
use tracing_futures::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

#[derive(Debug, StructOpt, Clone, Copy)]
#[structopt(name = "tracing-test", about = "Test tracing client/server")]
enum Opt {
    /// Start the tracing server.
    Server,
    /// Call the server with a client.
    Client,
}

fn set_subscriber() -> anyhow::Result<Uninstall> {
    global::set_text_map_propagator(Propagator::new());

    let (tracer, uninstall) = opentelemetry_jaeger::new_pipeline()
        .with_service_name("tracing test")
        .install()?;

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    let subscriber = Registry::default().with(telemetry);
    subscriber::set_global_default(subscriber)?;

    Ok(uninstall)
}

async fn handle(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let headers: HashMap<String, String> = req
        .headers()
        .into_iter()
        .map(|(hn, hv)| (hn.to_string(), hv.to_str().unwrap().to_string()))
        .collect();

    // The span created here should have client's span as the parent.
    let span = tracing::trace_span!("server handle", "otel.kind" = "server");

    let cx = global::get_text_map_propagator(|propagator| propagator.extract(&headers));
    span.set_parent(cx);

    let _guard = span.enter();
    thread::sleep(Duration::from_millis(300));

    do_something();

    Ok(Response::new(Body::from("hello, world!")))
}

fn do_something() {
    let span = tracing::trace_span!("doing something", "otel.kind" = "server");
    let _guard = span.enter();

    thread::sleep(Duration::from_millis(100));
}

async fn bind_server() -> anyhow::Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let make_service = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });
    let server = Server::bind(&addr).serve(make_service);

    println!("Listening on 127.0.0.1:3000");
    server.await?;

    Ok(())
}

async fn call_server() -> anyhow::Result<()> {
    // This should be the parent span.
    let span = tracing::trace_span!("client handle", "otel.kind" = "client");
    let cx = span.context();

    let mut headers = HashMap::new();
    global::get_text_map_propagator(|propagator| propagator.inject_context(&cx, &mut headers));

    let client = Client::new();

    let request = Request::builder()
        .method("GET")
        .uri(Uri::from_static("http://localhost:3000"));

    let request = headers
        .into_iter()
        .fold(request, |acc, (key, val)| acc.header(&key, &val));

    let res = client.request(request.body(Body::empty())?).instrument(span).await?;

    println!("status: {}", res.status());

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _guard = set_subscriber()?;

    match Opt::from_args() {
        Opt::Server => bind_server().await?,
        Opt::Client => call_server().await?,
    }

    Ok(())
}

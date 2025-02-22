mod config;
mod dbus;
mod extensions;
mod kinematics;
mod printer;
mod server;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::fs::OpenOptions;
use tokio::sync::RwLock;

pub const VERSION: (u8, u8, u8) = (0, 0, 1);
pub const API_VERSION: (u8, u8, u8) = (0, 0, 1);
pub const DEFAULT_HTTP_PORT: u16 = 8080;

lazy_static::lazy_static! {
    pub static ref INSTANCES: RwLock<HashMap<String, Arc<printer::Instance>>> = RwLock::new(HashMap::new());
}

#[tokio::main]
pub async fn main() {
    // parse command line arguments
    let cli_args = clap::Command::new("Gantry")
        .about("3D printer firmware")
        .version("v0.0.1")
        .arg(clap::arg!(-g --gantry_path <PATH> "gantry path, default $Home/.gantry"))
        .arg(clap::arg!(-p --port <PORT> "port for http server, default is port 80"))
        .get_matches();

    let port = cli_args
        .get_one::<u16>("port")
        .cloned()
        .unwrap_or(DEFAULT_HTTP_PORT);
    let gantry_path = cli_args
        .get_one::<PathBuf>("gantry_path")
        .cloned()
        .unwrap_or({
            // get home directory
            let g = dirs::home_dir()
                .expect("home directory not found")
                .join(".gantry");
            // create directory if not exist
            if !g.exists() {
                std::fs::create_dir(&g).expect("failed to create directory .gantry");
                std::fs::create_dir(g.join("themes")).expect("failed to create directry themes");
            }
            g
        })
        .canonicalize()
        .expect("path error");

    // open the config file in write mode
    let config_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(gantry_path.join("Gantry.toml"))
        .await
        .expect(&format!(
            "cannot open file '{}'",
            gantry_path.join("Gantry.toml").display()
        ));

    // parse config file
    let config = config::GantryConfig::parse(config_file).await.unwrap();

    // construct root dbus service
    let dbus = zbus::connection::Builder::session()
        .expect("failed to connect dbus")
        .name("org.gantry.ThreeD")
        .expect("gantry is already running")
        .build()
        .await
        .expect("failed to connect dbus");

    // get the dbus object server
    let obj_server = dbus.object_server();
    // serve server service
    obj_server
        .at("/org/gantry/server", dbus::Service::new())
        .await
        .unwrap();

    // spawn instances
    for (i, (name, inst_cfg)) in config.instances.into_iter().enumerate() {
        let inst = Arc::new(
            printer::Instance::create(i, name.clone(), inst_cfg, gantry_path.clone()).await,
        );

        // create dbus service
        let dbus_service = inst.clone().create_dbus_service();

        // register dbus interface
        let _ = obj_server
            .at(format!("/org/gantry/instance{}", i), dbus_service)
            .await;

        // add instance to global
        INSTANCES.write().await.insert(name, inst);
    }

    // construct axum server
    let app = axum::Router::<()>::new()
        .route(
            "/",
            axum::routing::get(|| async {
                axum::response::Html(include_str!("../../gantry-web.html"))
            }),
        )
        .route(
            "/gantry-web.css",
            axum::routing::get(|| async {
                (
                    [("content-type", "text/css")],
                    include_str!("../../gantry-web.css"),
                )
            }),
        )
        .route(
            "/gantry-web.js",
            axum::routing::get(|| async {
                (
                    [("content-type", "text/javascript")],
                    include_str!("../../gantry-web.js"),
                )
            }),
        )
        .nest("/server", server::create_service_router())
        .nest("/printer", printer::create_service_router());

    // run our app with hyper, listening globally
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("failed to bind TCP port");

    axum::serve(listener, app).await.unwrap();
}

mod config;
mod dbus;
mod extensions;
mod printer;
mod kinematics;
mod server;

use std::{path::{Path, PathBuf}, sync::Arc};

use clap::Parser;
use tokio::fs::OpenOptions;

/// 3D printer firmware
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Arguments {
    /// path contains all the configs and packages
    #[arg(long, short, default_value = "~/.gantry")]
    pub gantry_path: PathBuf,
    /// port of http server
    #[arg(long, short, default_value = "3000")]
    pub port: usize
}

#[tokio::main]
pub async fn main() {
    // parse command line arguments
    let args = Arguments::parse();

    // create configuration dir if not exist
    if args.gantry_path.as_path() == Path::new("~/.gantry") {
        // check if the default gantry path exist
        if !args.gantry_path.exists() {
            let _ = std::fs::create_dir("~/.gantry");
        }
    }

    // open the config file in write mode
    let config_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(args.gantry_path.join("gantry.toml"))
        .await
        .expect(&format!(
            "cannot open file '{}'",
            args.gantry_path.join("gantry.toml").display()
        ));

    // parse config file
    let config = config::GantryConfig::parse(config_file).await.unwrap();

    // construct root dbus service
    let dbus = zbus::connection::Builder::session()
        .expect("failed to connect dbus")
        .name("org.gantry.ThreeD")
        .expect("gantry is already running")
        .serve_at("/org/gantry/server", dbus::Service::new())
        .unwrap()
        .build()
        .await
        .expect("failed to connect dbus");

    let obj_server = dbus.object_server();

    // construct axum server
    let mut app = axum::Router::<()>::new()
    .nest("server", server::create_service_router());

    // spawn instances
    for (i, (name, inst_cfg)) in config.instances.into_iter().enumerate() {
        let inst = Arc::new(printer::Instance::create(i, name, inst_cfg, args.gantry_path.clone()));

        // create dbus service
        let dbus_service = inst.clone().create_dbus_service();

        // register dbus interface
        let _ = obj_server
            .at(format!("/org/gantry/instance{}", i), dbus_service)
            .await;

        // create http service
        let printer_router = inst.clone().create_axum_router();

        // register rest api
        app = app.nest(&format!("instance{}", i), printer_router);
    }

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
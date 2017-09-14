extern crate shio;

use shio::prelude::*;
use shio::handlers::{StaticFile, Configuration};

fn hello_world(_: Context) -> Response {
    Response::with("Hello World!\n")
}

fn hello(ctx: Context) -> Response {
    Response::with(format!("Hello, {}!", &ctx.get::<Parameters>()["name"]))
}

fn main() {
    Shio::default()
        .route((Method::Get, "/", hello_world))
        .route((Method::Get, "/{name}", hello))
        .route((Method::Get, "/static/{filepath:.*}", 
            StaticFile::new("examples/static_files/static/", Configuration::new().num_threads(2))
        ))
        .run(":7878")
        .unwrap();
}

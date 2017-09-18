extern crate shio;

use shio::prelude::*;

fn hello_world(_: Context) -> Response {
    Response::with("Hello World!\n")
}

fn hello(ctx: Context) -> Response {
    Response::with(format!("Hello, {}!", &ctx.get::<Parameters>()["name"]))
}

fn main() {
    // This needs helps for normalize static paths for avoid traversal attacks
    Shio::default()
        .route((Method::GET, "/", hello_world))
        .route((Method::GET, "/{name}", hello))
        .route((Method::GET, "/static/{filepath:.*}", 
            |ctx: Context| {
                Response::with(File::open(&ctx, "examples/static_files/static/", &ctx.get::<Parameters>()["filepath"]))
            }
        ))
        .route((Method::HEAD, "/static/{filepath:.*}",
            |ctx: Context| {
                Response::with(File::head(&ctx, "examples/static_files/static/", &ctx.get::<Parameters>()["filepath"]))
            }
        ))
        .run(":7878")
        .unwrap();
}

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
                let filebase : &str = &ctx.get::<Parameters>()["filepath"];
                let filename = "examples/static_files/static/".to_owned() + filebase;
                Response::with(File::open(&ctx, filename))
            }
        ))
        .route((Method::HEAD, "/static/{filepath:.*}",
            |ctx: Context| {
                let filebase : &str = &ctx.get::<Parameters>()["filepath"];
                let filename = "examples/static_files/static/".to_owned() + filebase;
                Response::with(File::head(&ctx, filename))
            }
        ))
        .run(":7878")
        .unwrap();
}

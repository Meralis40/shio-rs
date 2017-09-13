//! Handler for sending static files.
use hyper::{self, Method, StatusCode, header};
use std::path::PathBuf;
use rayon::{ThreadPool, Configuration};
use futures::future;
use std;

use handler::Handler;
use context::Context;
use response::Response;
use ext::BoxFuture;
use router::Parameters;

/// Handler for static files
pub struct StaticFile {
    root_path: PathBuf,
    pool: ThreadPool,
    get_path: Box<Fn(&Context) -> String + 'static + Send + Sync>,
}

fn default_get_path(ctxt: &Context) -> String {
    (&ctxt.get::<Parameters>()["filepath"]).to_owned()
}

impl StaticFile {
    /// Create a new `StaticFile` serving files from `root`
    pub fn new<P: Into<PathBuf>>(root: P, threadpool_config: Configuration) -> StaticFile {
        StaticFile {
            root_path: root.into(),
            pool: ThreadPool::new(threadpool_config).unwrap(),
            get_path: Box::new(default_get_path),
        }
    }

    /// Set the function that permit to retreive the path requested.
    ///
    /// By default, the handler assume that it is after a `Router`
    /// with a route that have a "filepath" parameter, like "/static/{filepath:.*}"
    ///
    /// You are not required to normalize the path (it's done by the handler itself)
    pub fn set_retreive_path<F>(&mut self, f: F)
    where
        F: Fn(&Context) -> String,
        F: Send + Sync + 'static
    {
        self.get_path = Box::new(f);
    }

    /// Provide normalization for path, and reduce risk of traversal attacks
    fn normalize(path: String) -> PathBuf {
        let mut normalized_path = PathBuf::new();

        for sp in path.split('/').filter(|x| x.len() != 0) {
            if sp == "." {
                // do nothing
            } else if sp == ".." {
                normalized_path.pop();
            } else {
                normalized_path.push(sp);
            }
        }

        normalized_path
    }
}

impl Handler for StaticFile {
    type Result = BoxFuture<Response, hyper::Error>;

    fn call(&self, ctx: Context) -> Self::Result {
        let base_path = StaticFile::normalize((self.get_path)(&ctx));
        let mut real_path = self.root_path.clone();
        real_path.push(base_path);

        let notfound = || Box::new(future::ok(Response::with(StatusCode::NotFound)));

        let is_head = match *ctx.method() {
            Method::Get => false,
            Method::Head => true,
            _ => {
                let response = Response::build()
                    .status(StatusCode::MethodNotAllowed)
                    .header(header::Allow(vec![Method::Get, Method::Head]))
                    .into();
                return Box::new(future::ok(response));
            }
        };

        let len = {
            let metadata = match std::fs::metadata(&real_path) {
                Ok(x) => x,
                Err(_) => {
                    // TODO: maybe another response
                    return notfound();
                }
            };

            if !metadata.is_file() {
                return notfound();
            }

            metadata.len()
        };

        if is_head {
            let response = Response::build()
                .status(StatusCode::Ok)
                .header(header::ContentLength(len))
                .into();
            return Box::new(future::ok(response));
        }

        let file = match std::fs::File::open(real_path) {
            Ok(file) => file,
            Err(_) => {
                // TODO: better error
                return notfound();
            }
        };

        let (sender, body) = hyper::Body::pair();

        self.pool.spawn(move || {
            use std::io::BufRead;
            use futures::Sink;

            let mut reader = std::io::BufReader::new(file);
            let mut sender = sender.wait();

            'readloop: loop {
                let data ={
                    let buff = match reader.fill_buf() {
                        Err(e) => {
                            // todo : log error
                            let _ = sender.send(Err(hyper::Error::from(e)));
                            break 'readloop;
                        },
                        Ok(buf) => buf,
                    };

                    Vec::from(buff)
                };
                reader.consume(data.len());
                let _ = sender.send(Ok(hyper::Chunk::from(data)));
            }

            let _ = sender.flush();
        });

        let response = Response::build()
            .status(StatusCode::Ok)
            .header(header::ContentLength(len))
            .body(body);
        Box::new(future::ok(response))
    }
}


use std::sync::{Arc};
use hyper;
use hyper::server::{Http, Request, Response, Service};
use futures;
use futures::future::{FutureResult};

pub struct IceServer {
    context: Arc<Context>
}

struct Context {
}

struct HttpService {
    context: Arc<Context>
}

impl IceServer {
    pub fn new() -> IceServer {
        return IceServer {
            context: Arc::new(Context {
            })
        }
    }

    pub fn listen(&self, addr: &str) {
        let addr = addr.parse().unwrap();
        let ctx = self.context.clone();

        let server = Http::new().bind(&addr, move || Ok(HttpService {
            context: ctx.clone()
        })).unwrap();

        server.run().unwrap();
    }
}

impl Service for HttpService {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = FutureResult<Response, hyper::Error>;

    fn call(&self, req: Request) -> Self::Future {
        println!("{}", req.remote_addr().unwrap());

        futures::future::ok(
            Response::new().with_body("Hello world")
        )
    }
}

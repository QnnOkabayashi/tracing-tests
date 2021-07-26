use crate::{request_error, request_info, request_warn};
use tide::{self, Middleware, Next, Request};
use tracing::{self, field::display, trace_span, Instrument};

// Modeled after:
// https://docs.rs/tide/0.16.0/src/tide/log/middleware.rs.html#23-96

pub struct TreeMiddleware {}

struct TreeMiddlewareFinished;

impl TreeMiddleware {
    pub fn new() -> Self {
        TreeMiddleware {}
    }

    // This method is only called when `instrument`ed.
    async fn log<'a, State: Clone + Send + Sync + 'static>(
        &'a self,
        mut req: Request<State>,
        next: Next<'a, State>,
    ) -> tide::Result {
        if req.ext::<TreeMiddlewareFinished>().is_some() {
            return Ok(next.run(req).await);
        }
        req.set_ext(TreeMiddlewareFinished);

        let path = req.url().path().to_string();
        let method = req.method();

        // Shadow owners
        let path = path.as_str();
        let method = method.as_ref();

        request_info!(http.method = method, path, "Request received");

        let response = next.run(req).await;
        let status = response.status();

        if status.is_server_error() {
            if let Some(error) = response.error() {
                request_error!(
                    message = display(error),
                    error_type = error.type_name().unwrap_or("?"),
                    status = format_args!("{} - {}", status as u16, status.canonical_reason()),
                    "Internal error -> Response sent"
                );
            } else {
                request_error!(
                    status = format_args!("{} - {}", status as u16, status.canonical_reason()),
                    "Internal error -> Response sent"
                );
            }
        } else if status.is_client_error() {
            if let Some(error) = response.error() {
                request_warn!(
                    message = display(error),
                    error_type = error.type_name().unwrap_or("?"),
                    status = format_args!("{} - {}", status as u16, status.canonical_reason()),
                    "Client error --> Response sent"
                );
            } else {
                request_warn!(
                    status = format_args!("{} - {}", status as u16, status.canonical_reason()),
                    "Client error --> Response sent"
                );
            }
        } else {
            request_info!(
                status = format_args!("{} - {}", status as u16, status.canonical_reason()),
                "--> Response sent"
            );
        }

        Ok(response)
    }
}

#[async_trait::async_trait]
impl<State: Clone + Send + Sync + 'static> Middleware<State> for TreeMiddleware {
    async fn handle(&self, req: Request<State>, next: Next<'_, State>) -> tide::Result {
        self.log(req, next)
            .instrument(trace_span!("tide-request"))
            .await
    }
}

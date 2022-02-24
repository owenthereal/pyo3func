mod function;

use function::*;
use std::io::Error;
use std::sync::Arc;
use tonic::{transport::Server, Request, Response, Status};

use pyo3::prelude::*;
use pyo3::types::{PyFunction, PyString, PyTuple};

#[pyfunction]
fn start_function(py: Python, f: Py<PyFunction>) -> PyResult<()> {
    let addr = "127.0.0.1:50051".parse().unwrap();
    let func = Function {
        callback: Arc::new(f),
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    let serve = Server::builder()
        .add_service(function_server::FunctionServer::new(func))
        .serve(addr);
    rt.block_on(serve).unwrap();

    Ok(())
}

#[pymodule]
fn pyo3func(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(start_function, m)?)?;
    Ok(())
}

pub struct Function {
    callback: Arc<Py<PyFunction>>,
}

#[tonic::async_trait]
impl function_server::Function for Function {
    async fn process(
        &self,
        request: Request<FunctionRequest>,
    ) -> Result<Response<FunctionResponse>, Status> {
        println!("Got a request: {:?}", request);

        let callback = self.callback.clone();
        let (tx, rx) = futures::channel::oneshot::channel::<String>();

        println!("before with_gil");
        let _ = Python::with_gil(|py| {
            println!("in gil");
            let args = PyTuple::new(py, [PyString::new(py, "hello")]);
            let ret = callback
                .call1(py, args)
                .unwrap()
                .extract::<String>(py)
                .unwrap();
            println!("{}", ret);
            let _ = tx.send(ret);

            Ok::<(), Error>(())
        });
        println!("after with_gil");

        let value = rx
            .await
            .map_err(|err| Status::internal(format!("Failed to call Python: {:?}", err)))?;

        let reply = FunctionResponse { value: value };
        Ok(Response::new(reply))
    }
}

use pyo3::{PyErr, Python};
use std::sync::OnceLock;
use tokio::sync::*;
use tracing::log::*;

static PYTHON_THREAD: OnceLock<std::thread::JoinHandle<()>> = OnceLock::new();
pub static PYTHON_CHANNEL: OnceLock<mpsc::Sender<Task>> = OnceLock::new();

pub type Result<T> = std::result::Result<T, String>;

pub enum Task {
    Echo(String),
    ScorePromptInjection(String, oneshot::Sender<Result<f64>>),
    ScoreToxicity(String, oneshot::Sender<Result<f64>>),
    PII(String, oneshot::Sender<Result<Vec<crate::ml::pii::PII>>>),
    #[cfg(test)]
    Test(i32, oneshot::Sender<i32>),
    Stop,
}

impl Task {
    pub async fn send(self) {
        PYTHON_CHANNEL.get()
            .expect("python_thread must be initialized")
            .send(self).await
            .expect("python_thread should be running");
    }
}

fn python_thread(mut rx: tokio::sync::mpsc::Receiver<Task>) {
    Python::with_gil(|py| {
        while let Some(python_task) = rx.blocking_recv() {
            let pool = unsafe { py.new_pool() };
            let py = pool.python();

            use Task::*;
            match python_task {
                Echo(s) => {
                    println!("{}", s);
                },
                ScorePromptInjection(s, tx) => {
                    crate::ml::prompt_injection::score_prompt_injection_py(
                        py, s, tx
                    );
                },
                ScoreToxicity(s, tx) => {
                    crate::ml::toxicity::score_toxicity_py(
                        py, s, tx
                    );
                },
                PII(s, tx) => {
                    crate::ml::pii::extract_pii_py(
                        py, s, tx
                    );
                },
                #[cfg(test)]
                Test(i, tx) => {
                    tx.send(i).unwrap();
                },
                Stop => break,
            }
        }
        info!("Closing python thread");
    });
}

pub fn init() {
    // Initialize python interpreter
    pyo3::prepare_freethreaded_python();

    pyo3_pylogger::register("sonnylabs::py");

    Python::with_gil(|py| {
        py.run(
            concat!(
                // fix ctrl-c handling
                "import signal\n",
                "signal.signal(signal.SIGINT, signal.SIG_DFL)\n",
                // set default logging
                "import logging\n",
                "logging.basicConfig(level=logging.INFO)\n"
            ),
            None,
            None,
        )
    })
    .expect("Signal and logging setup should succeed");
}

// Need to call this after other codes are initialized
pub fn init_python_thread() {
    let (tx, rx) = tokio::sync::mpsc::channel::<Task>(1024);
    PYTHON_CHANNEL.set(tx).expect("python::init() should be called only once");
    let handle = std::thread::spawn(move || python_thread(rx));
    PYTHON_THREAD.set(handle).expect("python::init() should be called only once");
}

pub fn format_error(py: Python<'_>, err: &PyErr) -> String {
    let value = err.value(py).str();
    let mut value_string = "<?>";
    if let Ok(s) = value {
        value_string = s.to_str().unwrap_or("<?>");
    }

    let mut tb_string = " - ".into();
    if let Some(tb) = err.traceback(py) {
        tb_string = tb.format().unwrap_or(" <-> ".into())
    }
    format!("Error: {value_string}\nTraceback: {tb_string}")
}

pub fn log_error_and_convert(py: Python<'_>, err: PyErr) -> String {
    error!("{}", format_error(py, &err));
    match err.value(py).str() {
        Ok(s) => s.to_str().unwrap_or("<?>").into(),
        Err(_) => "<?>".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_python_thread() {
        init_python_thread();

        assert!(PYTHON_THREAD.get().is_some());
        assert!(PYTHON_CHANNEL.get().is_some());

        assert!(!PYTHON_CHANNEL.get().unwrap().is_closed());

        let (tx, rx) = oneshot::channel();
        Task::Test(99, tx).send().await;
        let r = rx.await.unwrap();
        assert!(r == 99);

        Task::Stop.send().await;

        PYTHON_CHANNEL.get().unwrap().closed().await;
        loop {
            if PYTHON_THREAD.get().unwrap().is_finished() {
                break;
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }
}

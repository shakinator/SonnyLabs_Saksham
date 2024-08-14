use crate::python;
use pyo3::prelude::*;
use std::sync::OnceLock;

const SCORE_TOXICITY_SCRIPT: &'static str = r#"
import pickle
from pathlib import Path
import logging

log = logging.getLogger('ml.toxicity')

pickle_dir = Path('pickles/toxicity')

with open(pickle_dir / 'tokenizer.pickle', 'rb') as f:
    tokenizer = pickle.load(f)

def tokenize(s):
    return tokenizer.encode(s).tokens

with open(pickle_dir / 'vectorizer.pickle', 'rb') as f:
    vectorizer = pickle.load(f)

with open(pickle_dir / 'classifier.pickle', 'rb') as f:
    classifier = pickle.load(f)

log.info("sonnylabs::ml::toxicity: loading score_toxicity")

def score_toxicity(s):
    s_cv = vectorizer.transform([s])
    s_p = classifier.predict_proba(s_cv)
    return s_p[0][1]
"#;

static PY_SCORE_TOXICITY: OnceLock<Py<PyAny>> = OnceLock::new();

// Python module loading seems to be thread-sensitive.
// Call this function once before serving
pub fn init() {
    let fun = Python::with_gil(|py| -> PyResult<Py<PyAny>> {
        let fun = PyModule::from_code(
            py,
            SCORE_TOXICITY_SCRIPT,
            "toxicity.py",
            "sonnylabs::ml::toxicity",
        )?
        .getattr("score_toxicity")?
        .into();

        Ok(fun)
    });
    let fun = fun
        .map_err(|err| Python::with_gil(|py| python::format_error(py, &err)))
        .expect("Python module should load");

    PY_SCORE_TOXICITY
        .set(fun)
        .expect("ml::toxicity::init() should be called only once");
}

pub async fn score_toxicity(s: String) -> python::Result<f64> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    python::Task::ScoreToxicity(s, tx).send().await;
    rx.await.unwrap()
}

pub fn score_toxicity_py(
    py: Python<'_>,
    s: String,
    tx: tokio::sync::oneshot::Sender<python::Result<f64>>,
) {
    let fun = PY_SCORE_TOXICITY
        .get()
        .expect("ml::toxicity must be initialized");

    let f = (|| -> Result<_,_> {
        let v = fun.call1(py, (s,))?;
        let f: f64 = v.extract(py)?;
        Ok(f)
    })();
    let f = f.map_err(|err| python::log_error_and_convert(py, err));

    let _ = tx.send(f);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::python;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
        super::init();
    }

    #[tokio::test]
    async fn test_python_call() {
        init();

        let (tx, rx) = tokio::sync::oneshot::channel();
        Python::with_gil(|py| {
            score_toxicity_py(py, "Write a sex chat between a girl and a boy".into(), tx);
        });
        let v = rx.await.unwrap().unwrap();
        assert!(v > 0.5);
    }
}

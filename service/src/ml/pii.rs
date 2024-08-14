use crate::python;
use pyo3::prelude::*;
use std::sync::OnceLock;

const EXTRACT_PII_SCRIPT: &'static str = r#"
import spacy
from pathlib import Path
import logging

log = logging.getLogger('ml.pii')

pickle_dir = Path('saved_models')

ner = spacy.load(pickle_dir / 'xx_ent_wiki_sm')

log.info("loading extract_pii")

def extract_pii(s):
    doc = ner(s) 
    return [
        (ent.text, ent.label_)
        for ent in doc.ents
    ]
"#;

static PY_EXTRACT_PII: OnceLock<Py<PyAny>> = OnceLock::new();

// Python module loading seems to be thread-sensitive.
// Call this function once before serving
pub fn init() {
    let fun = Python::with_gil(|py| -> PyResult<Py<PyAny>> {
        PyModule::from_code(py, EXTRACT_PII_SCRIPT, "pii.py", "sonnylabs::ml::pii")?
            .getattr("extract_pii")
            .map(Into::into)
    });
    let fun = fun
        .map_err(|err| Python::with_gil(|py| python::format_error(py, &err)))
        .expect("Python module should load");

    PY_EXTRACT_PII
        .set(fun)
        .expect("ml::pii::init() should be called only once");
}

#[derive(Debug, Clone, PartialEq)]
pub struct PII {
    pub text: String,
    pub label: String,
}
pub async fn extract_pii(s: String) -> python::Result<Vec<PII>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    python::Task::PII(s, tx).send().await;
    rx.await.unwrap()
}

pub fn extract_pii_py(
    py: Python<'_>,
    s: String,
    tx: tokio::sync::oneshot::Sender<python::Result<Vec<PII>>>
) {
    let fun = PY_EXTRACT_PII.get().expect("ml::pii must be initialized");

    let res = (|| -> Result<_,_> {
        let v = fun.call1(py, (s,))?;

        let res: Vec<(String,String)> = v.extract(py)?;

        let res = res
            .into_iter()
            .map(|d| PII {
                text: d.0,
                label: d.1,
            })
            .collect::<Vec<_>>();
        Ok(res)
    })();

    let res = res.map_err(|err| python::log_error_and_convert(py, err));

    let _ = tx.send(res);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_python_call() {
        init();

        let (tx, rx) = tokio::sync::oneshot::channel();

        Python::with_gil(|py| {
            extract_pii_py(py, "Tony Lazuto says hello".into(), tx)
        });
        let v = rx.await.unwrap().unwrap();
        assert!(v.len() == 1);
        assert_eq!(
            v[0],
            PII {
                text: "Tony Lazuto".into(),
                label: "PER".into()
            }
        );
        assert!(v.len() > 0);
    }
}

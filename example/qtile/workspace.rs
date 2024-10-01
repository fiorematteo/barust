use async_trait::async_trait;
use barust::widgets::*;
use log::error;
use pyo3::{types::PyModule, Py, PyResult, Python};
use std::{collections::HashMap, fmt::Display};

pub struct QtileStatusProvider {
    python_module: Py<PyModule>,
    active_provider: ActiveProvider,
    group_count: HashMap<String, usize>,
}

impl std::fmt::Debug for QtileStatusProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt("QtileStatusProvider", f)
    }
}

#[async_trait]
impl WorkspaceStatusProvider for QtileStatusProvider {
    async fn update(&mut self) -> Result<()> {
        self.active_provider.update().await?;
        let Ok(group_count) = Python::with_gil(|py| -> PyResult<HashMap<String, usize>> {
            self.python_module
                .getattr(py, "windows")?
                .call0(py)?
                .extract::<HashMap<String, usize>>(py)
        }) else {
            error!("Failed to get group count");
            return Ok(());
        };
        self.group_count.clear();
        for (k, v) in group_count {
            self.group_count.insert(k, v);
        }
        Ok(())
    }

    async fn status(&self, workspace: &str, index: usize) -> WorkspaceStatus {
        let status = self.active_provider.status(workspace, index).await;
        if status == WorkspaceStatus::Active {
            status
        } else if self.group_count.contains_key(workspace) && self.group_count[workspace] > 0 {
            WorkspaceStatus::Used
        } else {
            WorkspaceStatus::Empty
        }
    }
}

impl QtileStatusProvider {
    pub async fn new() -> Result<Self> {
        let python_module = Python::with_gil(|py| -> PyResult<Py<PyModule>> {
            Ok(PyModule::from_code_bound(
                py,
                r#"from collections import Counter
from libqtile.command.client import CommandClient
import signal

signal.signal(signal.SIGINT, signal.SIG_DFL)
c = CommandClient()
def windows():
    windows = c.call("windows")
    return dict(Counter([(w["group"]) for w in windows if w["group"]]))"#,
                "",
                "",
            )?
            .into())
        })
        .map_err(Error::from)?;
        let active_provider = ActiveProvider::new()?;
        Ok(Self {
            python_module,
            active_provider,
            group_count: HashMap::new(),
        })
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    #[error("Ewmh")]
    Ewmh,
    Py(#[from] pyo3::PyErr),
}

impl From<Error> for WidgetError {
    fn from(value: Error) -> Self {
        WidgetError::custom(value)
    }
}

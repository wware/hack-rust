use std::path::PathBuf;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList, PyModule, PyType};

use crate::graph::Graph;
use crate::loader::load_graph_data;
use crate::types::{EdgeFilter, EntityNode, Node, StatementNode, TruthStatus};

#[pyclass(module = "bohemia_graph_native", name = "BohemiaGraph")]
pub struct PyBohemiaGraph {
    graph: Option<Graph>,
}

impl PyBohemiaGraph {
    fn graph(&self) -> PyResult<&Graph> {
        self.graph
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Graph has been closed"))
    }
}

fn optional_truth_status(truth: Option<&str>) -> PyResult<Option<TruthStatus>> {
    truth
        .map(|value| {
            TruthStatus::parse(value)
                .ok_or_else(|| PyRuntimeError::new_err(format!("invalid truth status: {value}")))
        })
        .transpose()
}

fn truth_statuses(truth_values: Option<Vec<String>>) -> PyResult<Option<Vec<TruthStatus>>> {
    truth_values
        .map(|values| {
            values
                .into_iter()
                .map(|value| {
                    TruthStatus::parse(&value).ok_or_else(|| {
                        PyRuntimeError::new_err(format!("invalid truth status: {value}"))
                    })
                })
                .collect()
        })
        .transpose()
}

fn entity_to_pydict(py: Python<'_>, entity: &EntityNode) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("node_kind", "entity")?;
    dict.set_item("id", &entity.id)?;
    dict.set_item("display_name", &entity.display_name)?;
    dict.set_item("aliases", PyList::new(py, &entity.aliases)?)?;
    dict.set_item("kind", &entity.kind)?;
    match entity.wiki_url.as_deref() {
        Some(url) => dict.set_item("wiki_url", url)?,
        None => dict.set_item("wiki_url", py.None())?,
    }
    Ok(dict.unbind())
}

fn statement_to_pydict(py: Python<'_>, stmt: &StatementNode) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("node_kind", "statement")?;
    dict.set_item("id", &stmt.id)?;
    dict.set_item("predicate", &stmt.predicate)?;
    dict.set_item("subject_id", &stmt.subject_id)?;
    dict.set_item("object_id", &stmt.object_id)?;
    dict.set_item("truth_status", stmt.truth_status.as_str())?;
    dict.set_item("story_id", &stmt.story_id)?;
    dict.set_item("paragraph_index", stmt.paragraph_index)?;
    dict.set_item("sentence_ids", PyList::new(py, &stmt.sentence_ids)?)?;
    match stmt.asserting_narrator_id.as_deref() {
        Some(id) => dict.set_item("asserting_narrator_id", id)?,
        None => dict.set_item("asserting_narrator_id", py.None())?,
    }
    dict.set_item("extraction_confidence", stmt.extraction_confidence)?;
    Ok(dict.unbind())
}

fn node_to_pydict(py: Python<'_>, node: &Node) -> PyResult<Py<PyDict>> {
    match node {
        Node::Entity(entity) => entity_to_pydict(py, entity),
        Node::Statement(stmt) => statement_to_pydict(py, stmt),
    }
}

fn statements_to_pylist(py: Python<'_>, stmts: Vec<&StatementNode>) -> PyResult<Py<PyList>> {
    let list = PyList::empty(py);
    for stmt in stmts {
        list.append(statement_to_pydict(py, stmt)?)?;
    }
    Ok(list.unbind())
}

fn sorted_layers(layers: Vec<std::collections::HashSet<String>>) -> Vec<Vec<String>> {
    layers
        .into_iter()
        .map(|layer| {
            let mut ids: Vec<String> = layer.into_iter().collect();
            ids.sort();
            ids
        })
        .collect()
}

#[pymethods]
impl PyBohemiaGraph {
    #[new]
    #[pyo3(signature = (lib_path=None))]
    fn new(lib_path: Option<&str>) -> Self {
        let _ = lib_path;
        Self {
            graph: Some(Graph::new(vec![])),
        }
    }

    #[classmethod]
    #[pyo3(signature = (lib_path=None))]
    fn find(_cls: &Bound<'_, PyType>, lib_path: Option<&str>) -> Self {
        Self::new(lib_path)
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc=None, _tb=None))]
    fn __exit__(
        &mut self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc: Option<&Bound<'_, PyAny>>,
        _tb: Option<&Bound<'_, PyAny>>,
    ) {
        self.close();
    }

    fn close(&mut self) {
        self.graph = None;
    }

    #[pyo3(signature = (entities, events, moments, triplets, sentence_cutoff=-1))]
    fn load(
        &mut self,
        entities: PathBuf,
        events: PathBuf,
        moments: PathBuf,
        triplets: PathBuf,
        sentence_cutoff: i32,
    ) -> PyResult<()> {
        let cutoff = if sentence_cutoff < 0 {
            None
        } else {
            Some(sentence_cutoff as u32)
        };

        let data = load_graph_data(&entities, &events, &moments, &triplets, cutoff)
            .map_err(PyRuntimeError::new_err)?;
        self.graph = Some(Graph::new(data.nodes));
        Ok(())
    }

    fn node_count(&self) -> PyResult<usize> {
        Ok(self.graph()?.by_id.len())
    }

    fn get(&self, py: Python<'_>, node_id: &str) -> PyResult<Option<Py<PyDict>>> {
        self.graph()?
            .get(node_id)
            .map(|node| node_to_pydict(py, node))
            .transpose()
    }

    fn describe(&self, node_id: &str) -> PyResult<String> {
        Ok(self.graph()?.describe(node_id))
    }

    #[pyo3(signature = (node_id, pred_type=None, truth=None))]
    fn edges_from(
        &self,
        py: Python<'_>,
        node_id: &str,
        pred_type: Option<&str>,
        truth: Option<&str>,
    ) -> PyResult<Py<PyList>> {
        let filter = EdgeFilter {
            pred_type: pred_type.map(str::to_string),
            truth: optional_truth_status(truth)?,
        };
        statements_to_pylist(py, self.graph()?.edges_from(node_id, &filter))
    }

    #[pyo3(signature = (node_id, pred_type=None, truth=None))]
    fn edges_to(
        &self,
        py: Python<'_>,
        node_id: &str,
        pred_type: Option<&str>,
        truth: Option<&str>,
    ) -> PyResult<Py<PyList>> {
        let filter = EdgeFilter {
            pred_type: pred_type.map(str::to_string),
            truth: optional_truth_status(truth)?,
        };
        statements_to_pylist(py, self.graph()?.edges_to(node_id, &filter))
    }

    #[pyo3(signature = (seeds, max_hops=2, truth_values=None))]
    fn bfs(
        &self,
        seeds: Vec<String>,
        max_hops: usize,
        truth_values: Option<Vec<String>>,
    ) -> PyResult<Vec<Vec<String>>> {
        let truth_values = truth_statuses(truth_values)?;
        let seed_refs: Vec<&str> = seeds.iter().map(|seed| seed.as_str()).collect();
        Ok(sorted_layers(
            self.graph()?
                .bfs(&seed_refs, max_hops, truth_values.as_deref()),
        ))
    }

    fn transitive_closure(&self, start: &str, predicate: &str) -> PyResult<Vec<String>> {
        let mut ids: Vec<String> = self
            .graph()?
            .transitive_closure(start, predicate)
            .into_iter()
            .collect();
        ids.sort();
        Ok(ids)
    }
}

#[pymodule(name = "_bohemia_graph_native")]
fn _bohemia_graph_native(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyBohemiaGraph>()?;
    Ok(())
}

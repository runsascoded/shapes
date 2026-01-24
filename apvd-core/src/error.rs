use crate::component;

#[derive(Debug, thiserror::Error)]
pub enum ShapeError {
    #[error("Unrecognized coordinate keys: {0:?}")]
    UnrecognizedCoordKeys(Vec<String>),
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum SceneError {
    #[error("Expected 1 container region within component {component_key:?} containing {child_key:?}, found {count}: {regions:?}")]
    ContainerRegionCount {
        component_key: component::Key,
        child_key: component::Key,
        count: usize,
        regions: Vec<String>,
    },

    #[error("Missing component for key: {0:?}")]
    MissingComponent(component::Key),

    #[error("Component {0:?} has no max depth (empty child keys but no depth computed)")]
    NoMaxDepth(component::Key),
}

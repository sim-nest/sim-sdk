use sim::{sim_class, sim_lib};

#[sim_lib(id = "broken-class", version = "0.1.0")]
mod broken_class {
    use super::sim_class;

    #[sim_class(name = "Point")]
    pub struct Point {
        x: f64,
        y: f64,
    }
}

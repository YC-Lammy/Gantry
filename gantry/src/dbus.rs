pub struct Service {}

impl Service {
    pub fn new() -> Self {
        Self {}
    }
}

#[zbus::interface(name = "org.gantry.server")]
impl Service {}

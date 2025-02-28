mod common;
mod api;
mod quake;

fn main() {
    // 谦者，众善之基；傲者，众恶之魁。
    // Humility is the foundation of all good; pride is the leader of all evil.
    // ------------------------------------------------------------------------
    env_logger::init();
    common::ArgParse::parse();
}
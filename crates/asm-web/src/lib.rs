//! Static dashboard generator for ASM datasets.

pub mod build;
pub mod collect;
pub mod figures;
pub mod pages;
pub mod serde;

pub use build::build_site;
pub use collect::{collect_site_data, SiteData};
pub use figures::{render_histogram_svg, FigureConfig};
pub use pages::{PageDescriptor, SiteConfig};

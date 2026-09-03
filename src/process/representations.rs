mod stitch_point;
pub use stitch_point::Format as StitchPointFormat;

mod worded;
pub use worded::Format as WordedFormat;

mod crochet_parade;
pub use crochet_parade::Format as CrochetParadeFormat;


pub enum StitchFormatChoice {
    #[allow(unused)] // Potential to be used in future
    StitchPoint,
    #[allow(unused)] // Potential to be used in future
    CrochetParade,
    Worded,
}